# General Assistant

A conversational agent that can answer questions, use tools, and maintain context across turns. The most common agent type — the foundation that other agents build on.

---

## Persona

The general assistant is helpful, direct, and honest about what it knows and doesn't know. It doesn't guess — if it's uncertain, it says so. If it can use a tool to find the answer, it does.

**Tone:** Conversational but concise. No filler. Answers the question, then stops.

**Expertise:** Generalist. Knows a little about everything, can use tools to go deeper.

---

## Capabilities

| Tool | Purpose | Example |
|------|---------|---------|
| `shell` | Run commands, check system state, process data | `shell("date")`, `shell("wc -l *.py")` |
| `read_file` | Read file contents for context | `read_file("README.md")` |
| `write_file` | Create or update files | `write_file("notes.md", content)` |
| `http_fetch` | Fetch content from a specific URL | `http_fetch("https://api.example.com/data")` |
| `web_search` | Search the web for information | `web_search("cognos language")`, `web_search("rust async", engine="duckduckgo")` |

The assistant doesn't decide which tools to use — the LLM does. The assistant defines which tools are *available*. The `think()` call with `tools=[...]` lets the LLM choose.

---

## Behavior

### Multi-turn conversation
The assistant maintains conversation history. Each user message is added to context, and each response builds on previous turns. This enables follow-up questions, references to earlier answers, and progressive refinement.

**When context grows too large:** The assistant compacts old history into a summary, preserving key facts and decisions while reducing token count. Recent turns are kept verbatim.

### Tool use
When the user asks something the assistant can't answer from knowledge alone, the LLM requests tool calls. The assistant executes them and feeds results back to the LLM.

**Tool selection is the LLM's job.** The assistant provides tool descriptions (flow docstrings), and the LLM decides which to call based on the user's request.

**Sequential vs parallel:** Most tool calls are sequential — the LLM sees each result before deciding the next step. For independent lookups, the LLM may request multiple tools in one response.

### When the assistant doesn't know
1. First: check if a tool can help (search, read a file, run a command)
2. If no tool helps: say "I don't know" honestly
3. Never fabricate facts. Never present uncertainty as confidence.

### Error handling
- Tool fails → report the error to the user, suggest alternatives
- LLM returns garbage → retry once with clearer prompt, then report failure
- Context too large → compact history, continue

---

## Output Contract

The general assistant has no fixed output type — it responds in natural language. However, when asked for structured output, it can use types:

```cognos
type TaskList:
    tasks: List[Task]

type Task:
    title: String
    priority: Priority
    done: Bool

type Priority: "high" | "medium" | "low"
```

Structured output is requested per-turn, not per-agent. The same assistant can answer "what time is it?" (free text) and "list my tasks" (structured).

---

## Memory

### Short-term: Conversation history
Maintained automatically by `think()`. Each call appends user input and assistant response. Compacted when context exceeds threshold.

### Long-term: Session persistence
Using `--session state.json`, the assistant preserves variables between runs. User preferences, accumulated notes, and task state survive restarts.

### Future: Semantic memory
`remember()` and `recall()` will enable the assistant to store and retrieve facts by meaning, not just by variable name. This is on the roadmap.

---

## Implementation

```cognos
import "lib/exec.cog"
import "lib/compact.cog"
import "lib/web_search.cog"

flow shell(command: String) -> String:
    "Run a shell command. Output limited to 50 lines."
    return __exec_shell__(f"{command} | head -50")

flow read_file(path: String) -> String:
    "Read the contents of a file"
    return read(file(path))

flow write_file(path: String, content: String) -> String:
    "Write content to a file"
    write(file(path), content)
    return f"Wrote {content.length} chars to {path}"

flow http_fetch(url: String) -> String:
    "Fetch content from a URL"
    return http.get(url)

flow main():
    system = "You are a helpful assistant. Be direct and concise. If you don't know something, say so. Use tools when they can help answer the question."
    model = "claude-sonnet-4-20250514"
    tools = ["shell", "read_file", "write_file", "http_fetch", "web_search"]
    max_history = 20

    write(stdout, "Assistant ready. Type your message.\n")

    loop:
        input = read(stdin)
        if input == "exit" or input == "quit":
            write(stdout, "Goodbye.")
            break

        # Compact history if it's getting long
        h = history()
        if h.length > max_history * 2:
            summary = compact_history(max_turns=max_history)
            write(stdout, "(context compacted)\n")

        # Think with tools — LLM decides which to use
        response = think(
            input,
            model=model,
            system=system,
            tools=tools
        )

        # Handle tool calls
        if response["has_tool_calls"]:
            # Execute tools and get final response
            executed = exec(response, tools=tools)
            # Feed results back to LLM for final answer
            final = think(
                f"Tool results:\n{executed[\"content\"]}",
                model=model,
                system=system,
                tools=tools
            )
            if final["has_tool_calls"]:
                # Second round of tools (max 2 rounds)
                executed2 = exec(final, tools=tools)
                final = think(
                    f"Tool results:\n{executed2[\"content\"]}",
                    model=model,
                    system=system
                )
            write(stdout, final["content"])
        else:
            write(stdout, response["content"])

        write(stdout, "\n")
```

### Usage

```bash
# Interactive conversation
cognos run general-assistant.cog --allow-shell

# With session persistence (remembers between runs)
cognos run general-assistant.cog --allow-shell --session assistant-state.json

# With tracing (for debugging)
cognos run general-assistant.cog --allow-shell --trace assistant.jsonl --trace-level full
```

### Testing

```bash
# Record a real session
cognos run general-assistant.cog --allow-shell --trace traces/assistant-session.jsonl

# Generate mock from trace
cognos trace-to-mock traces/assistant-session.jsonl > examples/mocks/assistant-test.json

# Replay deterministically
cognos test general-assistant.cog --env examples/mocks/assistant-test.json
```

---

## What this agent demonstrates

| Cognos Feature | How it's used |
|----------------|---------------|
| `think()` with tools | LLM chooses which tools to call |
| `exec()` from stdlib | Dispatches tool calls to flows via `invoke()` |
| `history()` / `compact_history()` | Conversation memory with compaction |
| `--session` | State persistence between runs |
| Flow docstrings | Tool descriptions for the LLM |
| `try/catch` (in exec) | Graceful tool failure handling |
| `--trace` | Full observability |
| `cognos test --env` | Deterministic replay testing |
