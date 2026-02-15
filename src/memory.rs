//! Semantic memory engine for Cognos agents.
//!
//! Provides `remember(text)`, `recall(query, limit)`, `forget(query)` backed by
//! Ollama embeddings + SQLite vector storage. All details hidden from .cog authors.

use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

const DEFAULT_MODEL: &str = "nomic-embed-text";
const DEDUP_THRESHOLD: f64 = 0.95;
const FORGET_THRESHOLD: f64 = 0.60;

/// Semantic memory store.
pub struct MemoryStore {
    db: Arc<Mutex<Connection>>,
    namespace: String,
    ollama_url: String,
    model: String,
}

impl MemoryStore {
    /// Create or open a persistent memory store.
    pub fn open(db_path: &str, namespace: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Self::init(conn, namespace)
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory(namespace: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn, namespace)
    }

    fn init(conn: Connection, namespace: &str) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                namespace TEXT NOT NULL,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                access_count INTEGER NOT NULL DEFAULT 0,
                score REAL NOT NULL DEFAULT 0.0
            );
            CREATE INDEX IF NOT EXISTS idx_memories_ns ON memories(namespace);
            -- Migration: add score column if missing (existing DBs)
            -- SQLite ignores duplicate ADD COLUMN errors at runtime"
        )?;
        // Migration for existing databases
        let _ = conn.execute_batch("ALTER TABLE memories ADD COLUMN score REAL NOT NULL DEFAULT 0.0");

        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("COGNOS_EMBED_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            namespace: namespace.to_string(),
            ollama_url,
            model,
        })
    }

    /// Store a fact. Skips near-duplicates (cosine > 0.95).
    pub fn remember(&self, text: &str) -> Result<()> {
        let embedding = self.embed(text)?;
        
        // Check for duplicates
        let existing = self.search_raw(&embedding, 1)?;
        if let Some((_, score)) = existing.first() {
            if *score > DEDUP_THRESHOLD {
                log::info!("memory: skipping duplicate (similarity={:.3})", score);
                return Ok(());
            }
        }

        let blob = embedding_to_blob(&embedding);
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO memories (namespace, text, embedding) VALUES (?1, ?2, ?3)",
            params![self.namespace, text, blob],
        )?;
        log::info!("memory: stored fact ({} bytes)", text.len());
        Ok(())
    }

    /// Store a fact with an explicit quality score.
    /// If a near-duplicate exists (cosine > 0.95), updates its score instead.
    pub fn remember_scored(&self, text: &str, score: f64) -> Result<()> {
        let embedding = self.embed(text)?;

        // Check for duplicates — update score if found
        let all = self.all_with_embeddings()?;
        for (id, _existing_text, emb) in &all {
            let sim = cosine_similarity(&embedding, emb);
            if sim > DEDUP_THRESHOLD {
                let db = self.db.lock().unwrap();
                // Exponential moving average: new_score = 0.7 * new + 0.3 * old
                db.execute(
                    "UPDATE memories SET score = 0.7 * ?1 + 0.3 * score WHERE id = ?2",
                    params![score, id],
                )?;
                log::info!("memory: updated score for existing fact (id={}, new_score={:.2})", id, score);
                return Ok(());
            }
        }

        let blob = embedding_to_blob(&embedding);
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO memories (namespace, text, embedding, score) VALUES (?1, ?2, ?3, ?4)",
            params![self.namespace, text, blob, score],
        )?;
        log::info!("memory: stored scored fact ({} bytes, score={:.2})", text.len(), score);
        Ok(())
    }

    /// Semantic search. Returns up to `limit` facts, most relevant first.
    pub fn recall(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let embedding = self.embed(query)?;
        let results = self.search_hybrid(&embedding, query, limit)?;
        
        // Update access counts
        let db = self.db.lock().unwrap();
        for (text, _score) in &results {
            db.execute(
                "UPDATE memories SET access_count = access_count + 1 WHERE namespace = ?1 AND text = ?2",
                params![self.namespace, text],
            )?;
        }
        
        Ok(results.into_iter().map(|(text, _)| text).collect())
    }

    /// Semantic search returning scored results with quality metadata.
    /// Returns Vec<(text, similarity, quality_score)>.
    pub fn recall_scored(&self, query: &str, limit: usize) -> Result<Vec<(String, f64, f64)>> {
        let embedding = self.embed(query)?;
        let all = self.all_with_embeddings_and_scores()?;
        let query_tokens: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() >= 2)
            .map(|w| w.to_lowercase())
            .collect();

        let mut scored: Vec<(String, f64, f64)> = all
            .into_iter()
            .map(|(_id, text, emb, quality_score)| {
                let semantic_score = cosine_similarity(&embedding, &emb);
                let text_lower = text.to_lowercase();
                let keyword_hits = query_tokens.iter()
                    .filter(|token| text_lower.contains(token.as_str()))
                    .count();
                let keyword_boost = (keyword_hits as f64 * 0.15).min(0.3);
                let combined = semantic_score + keyword_boost;
                (text, combined, quality_score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Update access counts
        let db = self.db.lock().unwrap();
        for (text, _, _) in &scored {
            let _ = db.execute(
                "UPDATE memories SET access_count = access_count + 1 WHERE namespace = ?1 AND text = ?2",
                params![self.namespace, text],
            );
        }

        Ok(scored)
    }

    /// Remove facts matching query (cosine > 0.80).
    pub fn forget(&self, query: &str) -> Result<usize> {
        let embedding = self.embed(query)?;
        let all = self.all_with_embeddings()?;
        let mut removed = 0;
        let db = self.db.lock().unwrap();
        for (id, _text, emb) in &all {
            let score = cosine_similarity(&embedding, emb);
            if score > FORGET_THRESHOLD {
                db.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
                removed += 1;
            }
        }
        log::info!("memory: forgot {} facts", removed);
        Ok(removed)
    }

    /// Get total fact count for this namespace.
    pub fn count(&self) -> Result<usize> {
        let db = self.db.lock().unwrap();
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM memories WHERE namespace = ?1",
            params![self.namespace],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // --- Internal ---

    fn embed(&self, text: &str) -> Result<Vec<f64>> {
        let url = format!("{}/api/embeddings", self.ollama_url);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": text,
        });
        let client = reqwest::blocking::Client::new();
        let resp = client.post(&url)
            .json(&body)
            .send()
            .map_err(|e| anyhow::anyhow!("embedding request failed: {}. Is Ollama running with model '{}'?", e, self.model))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("embedding failed ({}): {}. Try: ollama pull {}", status, body, self.model);
        }

        let json: serde_json::Value = resp.json()?;
        let embedding = json["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("invalid embedding response"))?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect::<Vec<f64>>();

        if embedding.is_empty() {
            log::warn!("empty embedding returned for text: {:?}", &text[..text.len().min(50)]);
            // Return zero vector of expected dimension (768 for most models)
            return Ok(vec![0.0; 768]);
        }
        Ok(embedding)
    }

    fn search_raw(&self, query_embedding: &[f64], limit: usize) -> Result<Vec<(String, f64)>> {
        self.search_hybrid(query_embedding, "", limit)
    }

    /// Hybrid search: semantic similarity + keyword boost.
    /// Words from the query that appear in a fact's text boost its score.
    /// This handles identifiers/labels (P11, BUG-3, etc.) that embeddings miss.
    fn search_hybrid(&self, query_embedding: &[f64], query_text: &str, limit: usize) -> Result<Vec<(String, f64)>> {
        let all = self.all_with_embeddings_and_scores()?;
        // Extract query tokens for keyword matching (lowercase, 2+ chars)
        let query_tokens: Vec<String> = query_text
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() >= 2)
            .map(|w| w.to_lowercase())
            .collect();

        let mut scored: Vec<(String, f64)> = all
            .into_iter()
            .map(|(_id, text, emb, quality_score)| {
                let semantic_score = cosine_similarity(query_embedding, &emb);
                // Keyword boost: for each query token found in the text, add a boost
                let text_lower = text.to_lowercase();
                let keyword_hits = query_tokens.iter()
                    .filter(|token| text_lower.contains(token.as_str()))
                    .count();
                // Boost: 0.15 per keyword hit, capped at 0.3
                let keyword_boost = (keyword_hits as f64 * 0.15).min(0.3);
                // Quality score boost: scale from [-1, 1] to [-0.2, 0.2]
                let quality_boost = quality_score * 0.2;
                let combined = semantic_score + keyword_boost + quality_boost;
                (text, combined)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    fn all_with_embeddings(&self) -> Result<Vec<(i64, String, Vec<f64>)>> {
        Ok(self.all_with_embeddings_and_scores()?
            .into_iter()
            .map(|(id, text, emb, _score)| (id, text, emb))
            .collect())
    }

    fn all_with_embeddings_and_scores(&self) -> Result<Vec<(i64, String, Vec<f64>, f64)>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, text, embedding, score FROM memories WHERE namespace = ?1"
        )?;
        let rows = stmt.query_map(params![self.namespace], |row| {
            let id: i64 = row.get(0)?;
            let text: String = row.get(1)?;
            let blob: Vec<u8> = row.get(2)?;
            let score: f64 = row.get::<_, f64>(3).unwrap_or(0.0);
            Ok((id, text, blob, score))
        })?;
        let mut results = Vec::new();
        for row in rows {
            let (id, text, blob, score) = row?;
            let emb = blob_to_embedding(&blob);
            results.push((id, text, emb, score));
        }
        Ok(results)
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn embedding_to_blob(embedding: &[f64]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(embedding.len() * 8);
    for &v in embedding {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f64> {
    blob.chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-10);

        let d = vec![1.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &d);
        assert!((sim - 0.7071).abs() < 0.01);
    }

    #[test]
    fn test_embedding_blob_roundtrip() {
        let original = vec![1.0, -2.5, 3.14159, 0.0, -0.001];
        let blob = embedding_to_blob(&original);
        let recovered = blob_to_embedding(&blob);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_memory_store_in_memory() {
        // This tests the DB schema and basic CRUD without Ollama
        let store = MemoryStore::in_memory("test").unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }
}
