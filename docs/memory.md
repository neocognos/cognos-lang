# Memory in Cognos

## Philosophy

Memory strategy is logic — it belongs in `.cog` files, not hardcoded in the runtime. The runtime provides primitives (`remember()`, `recall()`); your code decides *what* to store and *when* to retrieve.

## Current: List-Based History

For now, memory is a list that holds conversation history. Simple, predictable, sufficient for most agents:

```cognos
flow agent(input: String):
    history = []
    loop:
        history = history + [f"User: {input}"]
        context = history.join("\n")
        response = think(context, system="You are a helpful assistant.")
        history = history + [f"Assistant: {response}"]
        write(stdout, response)
```

## Future: Semantic Memory

When the runtime supports `remember()` and `recall()`, memory becomes intelligent:

```cognos
# memory.cog — reusable memory module

flow extract_facts(text: String) -> List[String]:
    facts = think(text, format="List[String]",
        system="Extract key facts. Be specific and self-contained.")
    return facts

flow memorize(text: String):
    facts = extract_facts(text)
    for fact in facts:
        remember(fact)
    write(stdout, f"Stored {facts.length} facts")

flow answer(question: String) -> String:
    context = recall(question)
    if context.length == 0:
        return think(question)
    return think(f"Context: {context}\n\nQuestion: {question}",
        system="Answer using the provided context.")
```

Then any agent imports it:

```cognos
import "memory.cog"

flow main(input: String):
    if input.starts_with("remember:"):
        memorize(input.replace("remember:", "").strip())
    else:
        write(stdout, answer(input))
```

## Design Principles

| Principle | Meaning |
|-----------|---------|
| **Strategy is logic** | What to store, when to retrieve — that's `.cog` code |
| **Engine is runtime** | Vector DB, embeddings, persistence — runtime builtins |
| **Swappable** | Different agents, different memory strategies, same engine |
| **Auditable** | Read the `.cog` file to see exactly what gets remembered |

## Roadmap

- [x] List-based conversation history
- [ ] `remember(fact: String)` — store a fact in semantic memory
- [ ] `recall(query: String)` → List[String] — retrieve relevant facts
- [ ] `forget(query: String)` — remove facts
- [ ] Memory consolidation (summarize + deduplicate)
- [ ] Memory persistence across sessions
- [x] `import "memory.cog"` — reusable memory module (import system implemented)
