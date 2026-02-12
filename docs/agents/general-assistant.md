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
import "../lib/web_search.cog"

type Action:
    type: String
    task_name: String

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

flow do_task(description: String) -> String:
    "Execute a complex task in the background"
    tools = ["shell", "read_file", "web_search"]
    response = think(description, model="claude-sonnet-4-20250514", system="Complete this task thoroughly.", tools=tools)
    if response.has_tool_calls:
        executed = exec(response, tools=tools)
        response = think(
            f"Task: {description}\nTool results:\n{executed.content}",
            model="claude-sonnet-4-20250514",
            system="Synthesize into a clear answer."
        )
    return response.content

flow handle_input(input: String, tasks: Map) -> Map:
    tools = ["shell", "read_file", "write_file", "web_search"]
    if input == "status":
        if tasks.length == 0:
            write(stdout, "No tasks running.\n")
        for name, handle in tasks:
            write(stdout, f"  ⏳ {name}: running\n")
        return tasks
    if input.starts_with("cancel "):
        name = input[7:].strip()
        try:
            cancel(tasks[name])
            tasks = remove(tasks, name)
            write(stdout, f"🛑 Cancelled '{name}'.\n")
        catch err:
            write(stdout, f"No task named '{name}'.\n")
        return tasks
    action = think(
        f"User said: \"{input}\"",
        format="Action",
        model="claude-sonnet-4-20250514",
        system="Classify intent. type='respond' for quick answers, type='spawn' for background tasks."
    )
    if action.type == "spawn":
        handle = async do_task(input)
        tasks[action.task_name] = handle
        write(stdout, f"⚡ Started '{action.task_name}' in background. Keep talking.\n")
        return tasks
    response = think(input, model="claude-sonnet-4-20250514", system="Be concise and helpful.", tools=tools)
    if response.has_tool_calls:
        executed = exec(response, tools=tools)
        final = think(f"Tool results:\n{executed.content}", model="claude-sonnet-4-20250514", system="Answer concisely.")
        write(stdout, f"{final.content}\n")
    else:
        write(stdout, f"{response.content}\n")
    return tasks

flow main():
    write(stdout, "🤖 Responsive Assistant\n")
    write(stdout, "Commands: status, cancel <name>, exit\n\n")
    tasks = {}
    loop:
        write(stdout, "> ")
        if tasks.length == 0:
            input = read(stdin)
            if input == "exit":
                break
            tasks = handle_input(input, tasks)
        else:
            select:
                branch:
                    input = read(stdin)
                    if input == "exit":
                        for name, handle in tasks:
                            cancel(handle)
                        break
                    tasks = handle_input(input, tasks)
                branch:
                    first_name = ""
                    first_handle = None
                    for name, handle in tasks:
                        if first_name == "":
                            first_name = name
                            first_handle = handle
                    result = await(first_handle)
                    write(stdout, f"\n✅ Task '{first_name}' complete:\n{result}\n")
                    tasks = remove(tasks, first_name)
    write(stdout, "Goodbye! 👋\n")
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
