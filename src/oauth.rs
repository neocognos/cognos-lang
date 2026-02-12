//! Anthropic OAuth PKCE flow for Claude Max subscription
//!
//! One-time browser login, then auto-refresh.
//! Token stored in ~/.cognos/oauth.json

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
const SCOPES: &str = "org:create_api_key user:profile user:inference";

#[derive(Serialize, Deserialize, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64, // epoch millis
}

fn token_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cognos/oauth.json")
}

/// Load saved token from disk
pub fn load_token() -> Option<OAuthToken> {
    let path = token_path();
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save token to disk
fn save_token(token: &OAuthToken) -> Result<()> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(token)?;
    std::fs::write(&path, data)?;
    Ok(())
}

/// Refresh an expired token
fn refresh_token(refresh: &str) -> Result<OAuthToken> {
    let client = reqwest::blocking::Client::new();
    let resp = client.post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": CLIENT_ID,
            "refresh_token": refresh,
        }))
        .send()?;

    if !resp.status().is_success() {
        let text = resp.text()?;
        bail!("Token refresh failed: {}", text);
    }

    let data: serde_json::Value = resp.json()?;
    let access = data["access_token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No access_token in refresh response"))?;
    let refresh_new = data["refresh_token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No refresh_token in refresh response"))?;
    let expires_in = data["expires_in"].as_u64().unwrap_or(28800);
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + (expires_in * 1000)
        - (5 * 60 * 1000); // 5 min buffer

    let token = OAuthToken {
        access_token: access.to_string(),
        refresh_token: refresh_new.to_string(),
        expires_at,
    };
    save_token(&token)?;
    Ok(token)
}

/// Get a valid access token, refreshing if needed
pub fn get_access_token() -> Result<String> {
    if let Some(token) = load_token() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if now < token.expires_at {
            return Ok(token.access_token);
        }

        // Try to refresh
        match refresh_token(&token.refresh_token) {
            Ok(new_token) => return Ok(new_token.access_token),
            Err(e) => {
                log::warn!("Token refresh failed: {}. Run 'cognos login' to re-authenticate.", e);
            }
        }
    }

    bail!("No valid Cognos OAuth token. Run 'cognos login' to authenticate with your Claude Max subscription.")
}

/// Generate PKCE challenge (RFC 7636)
fn generate_pkce() -> (String, String) {
    use sha2::{Sha256, Digest};
    use base64::Engine;

    // Generate 32 random bytes for verifier
    let mut random_bytes = [0u8; 32];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    // Mix time, pid, and address of local var for entropy
    let seed = now.as_nanos() ^ (std::process::id() as u128) ^ (&random_bytes as *const _ as u128);
    for (i, b) in random_bytes.iter_mut().enumerate() {
        let v = seed.wrapping_mul(6364136223846793005).wrapping_add(i as u128 * 1442695040888963407);
        *b = (v >> (i * 3)) as u8;
    }

    // Verifier: base64url-encoded random bytes (43-128 chars per spec)
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);

    // Challenge: base64url(SHA256(verifier))
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Interactive login flow — opens browser, user pastes code
pub fn login() -> Result<OAuthToken> {
    let (verifier, challenge) = generate_pkce();

    let auth_url = format!(
        "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        AUTHORIZE_URL, CLIENT_ID,
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(SCOPES),
        challenge, verifier
    );

    println!("\n🔐 Cognos OAuth Login");
    println!("Open this URL in your browser:\n");
    println!("  {}\n", auth_url);

    // Try to open browser
    let _ = std::process::Command::new("xdg-open").arg(&auth_url).spawn();

    println!("After authorizing, paste the code (format: code#state):");
    print!("> ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let parts: Vec<&str> = input.split('#').collect();
    let code = parts[0];
    let state = parts.get(1).unwrap_or(&"");

    // Exchange code for tokens
    let client = reqwest::blocking::Client::new();
    let resp = client.post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": CLIENT_ID,
            "code": code,
            "state": state,
            "redirect_uri": REDIRECT_URI,
            "code_verifier": verifier,
        }))
        .send()?;

    if !resp.status().is_success() {
        let text = resp.text()?;
        bail!("Token exchange failed: {}", text);
    }

    let data: serde_json::Value = resp.json()?;
    let access = data["access_token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No access_token"))?;
    let refresh = data["refresh_token"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No refresh_token"))?;
    let expires_in = data["expires_in"].as_u64().unwrap_or(28800);
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + (expires_in * 1000)
        - (5 * 60 * 1000);

    let token = OAuthToken {
        access_token: access.to_string(),
        refresh_token: refresh.to_string(),
        expires_at,
    };
    save_token(&token)?;

    println!("✅ Logged in! Token saved to ~/.cognos/oauth.json");
    println!("   Expires in {} hours. Auto-refresh enabled.", expires_in / 3600);

    Ok(token)
}
