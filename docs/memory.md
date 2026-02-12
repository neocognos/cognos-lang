# Memory in Cognos

## Philosophy

Memory strategy is logic — it belongs in `.cog` files, not hardcoded in the runtime. The runtime provides three primitives; your code decides *what* to store and *when* to retrieve.

## Interface

Three builtins. That's the entire surface:

```cognos
remember("Reza prefers terse output")        # store a fact
facts = recall("user preferences", limit=5)  # semantic search → List[String]
forget("outdated preference")                 # remove matching facts
```

Everything else is hidden inside the runtime:
- Embedding computation
- Vector storage and indexing
- Cosine similarity search
- Deduplication
- Namespace/agent isolation
- Database location
- Embedding model selection

## Architecture

```
┌─────────────────────────────────┐
│  .cog flow (agent logic)        │
│  remember(fact)                 │
│  facts = recall(query)          │
└──────────┬──────────────────────┘
           │ builtins
┌──────────▼──────────────────────┐
│  Memory Engine (Rust runtime)   │
│  - Embed: Ollama nomic-embed    │
│  - Store: SQLite + vector blob  │
│  - Search: cosine similarity    │
│  - Namespace: per agent         │
└─────────────────────────────────┘
```

## Builtins

### `remember(text: String)`

Store a fact in semantic memory. The runtime:
1. Computes an embedding via Ollama (`nomic-embed-text`)
2. Checks for near-duplicates (cosine > 0.95) — skips if duplicate
3. Stores text + embedding + timestamp in SQLite

```cognos
remember("project uses DeepSeek as primary cloud model")
remember("test suite has 215 unit tests and 71 harness tests")
```

### `recall(query: String, limit: Int = 5) → List[String]`

Retrieve relevant facts by semantic similarity. Returns plain strings, most relevant first.

```cognos
facts = recall("what models do we use")
# → ["project uses DeepSeek as primary cloud model", ...]

facts = recall("test coverage", limit=3)
# → top 3 matching facts
```

### `forget(query: String)`

Remove facts semantically matching the query (cosine > 0.8). Use to clean outdated information.

```cognos
forget("old API key configuration")
```

## Patterns

### Long-running agent with memory

```cognos
flow main():
    history = []
    loop:
        input = read(stdin)
        if input == none:
            break

        # Retrieve relevant context
        facts = recall(input, limit=5)
        recent = history[-5:]

        context = ""
        if facts.length > 0:
            context = f"Known facts:\n{facts.join(chr(10))}\n\n"
        context = context + f"Recent conversation:\n{recent.join(chr(10))}"

        response = think(f"{context}\n\nUser: {input}",
            model="deepseek-chat",
            system="You are a helpful assistant. Use known facts for context.")

        # Store important facts from the conversation
        remember(f"User said: {input}")

        history = history + [f"User: {input}", f"Assistant: {response.content}"]
        write(stdout, response.content)
```

### Periodic compaction

```cognos
flow compact(history: List) -> List:
    "Summarize old history into facts, return trimmed history"
    if history.length < 20:
        return history

    old = history[:15]
    summary = think(f"Summarize key facts from this conversation:\n{old.join(chr(10))}",
        model="deepseek-chat",
        system="Extract 3-5 key facts. One fact per line. Be specific.")

    for fact in summary.content.split("\n"):
        remember(fact.strip())

    return history[15:]
```

### Memory-augmented tool agent

```cognos
flow main():
    loop:
        input = read(stdin)
        if input == none:
            break

        if input.starts_with("remember:"):
            fact = input.replace("remember:", "").strip()
            remember(fact)
            write(stdout, f"Remembered: {fact}")
        elif input.starts_with("recall:"):
            query = input.replace("recall:", "").strip()
            facts = recall(query, limit=10)
            write(stdout, facts.join("\n"))
        else:
            facts = recall(input)
            response = think(f"Context: {facts.join(chr(10))}\n\nQuestion: {input}",
                model="deepseek-chat")
            write(stdout, response.content)
```

## Implementation Details (hidden from .cog authors)

| Component | Choice | Reason |
|-----------|--------|--------|
| Embedding model | `nomic-embed-text` via Ollama | Already running, zero new deps, 768 dims |
| Storage | SQLite via `rusqlite` | Single file, zero infra, ships with binary |
| Vector search | Brute-force cosine in Rust | Fast enough for <100K facts |
| Deduplication | Cosine > 0.95 threshold | Prevents storing same fact twice |
| Namespace | Agent name or `--memory-ns` flag | Isolation between agents |
| DB location | `~/.cognos/memory.db` or `--memory-db` | Sensible default, overridable |
| Testability | `MockEnv` uses in-memory SQLite | No disk I/O in tests |

## Design Principles

| Principle | Meaning |
|-----------|---------|
| **3 builtins** | `remember`, `recall`, `forget` — nothing else exposed |
| **Strategy is logic** | What to store, when to retrieve — that's `.cog` code |
| **Engine is runtime** | Embeddings, vectors, persistence — invisible to agent |
| **Zero config** | Works out of the box with Ollama running |
| **P11 compliant** | Memory is core to agents (think/act/observe/**remember**) |
