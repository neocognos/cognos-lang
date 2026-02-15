# Meta-Agent: LLM-Generated Flows

## The Problem

Every agent framework today follows the same pattern: put an LLM in a while loop, feed it tool results, hope it converges. This creates fundamental problems:

- **Context decay** — conversation history grows until it overflows or gets truncated, losing critical information
- **No control flow** — the LLM can't express conditionals, loops, or branching; it just reacts turn by turn
- **State lives in tokens** — every piece of information the agent "remembers" costs tokens every turn
- **Opaque reasoning** — you can't inspect, debug, or improve a 15-turn conversation where the model went wrong at turn 4

## The Approach

Instead of using an LLM as a reactive tool-caller, Cognos uses LLMs as **programmers that write programs which themselves can think**.

```
┌─────────────┐     Cognos source     ┌─────────────┐     result
│  Master LLM │ ──────────────────►   │   Runtime    │ ──────────►
│  (Opus)     │                       │   eval()     │
└─────────────┘                       └─────────────┘
       ▲                                     │
       │            task + context            │
       └──────────────────────────────────────┘
                  (on failure: retry)
```

1. **Master receives the task** — a bug report, feature request, or any problem
2. **Master generates a Cognos flow** — a task-specific program with `think()` calls where reasoning is needed, and deterministic logic everywhere else
3. **Runtime executes the flow** via `eval()` — tool calls, file I/O, and LLM reasoning all happen within the generated program
4. **On failure** — the master gets the error and generates a revised flow

## Why This Works

### State lives in variables, not conversation history
```
content = read_file("src/main.py")     # stored in variable, zero ongoing token cost
line_num = 42                           # precise, never forgotten
```
In a traditional agent, this information is buried in conversation history and costs tokens every turn until it gets truncated.

### Control flow is explicit
```
if "error" in test_output:
    fix = think(f"Fix this error: {test_output}")
    edit_file(path, fix)
else:
    write(stdout, "Tests pass, done")
```
Traditional agents can't express this — they have to re-decide the strategy every turn.

### Think() is surgical
The generated flow calls `think()` only where actual reasoning is needed:
```
# Deterministic: find files, read content
files = shell("find . -name '*.py' | head -20")
content = read_file("src/models.py")

# LLM reasoning: only for analysis and decisions
analysis = think(f"Where is the bug in this code?\n{content}")

# Deterministic: apply the fix
edit_file("src/models.py", old_text, new_text)
```
This minimizes LLM calls and maximizes reliability.

### It's inspectable and debuggable
The generated flow is saved to disk. You can:
- Read it to understand the strategy
- Modify it and re-run
- Use it as a template for similar problems

### It compounds through memory
```
# After a successful fix:
remember("For Django ORM bugs, check models.py and migrations/")

# Next time, the master recalls this:
past = recall("Django bug fixing strategy")
# → generates a better flow informed by past experience
```

## Architecture

### Components

- **`prompts/cognos-flow-generator.md`** — System prompt with the full Cognos language reference. Passed to the master LLM so it can write valid Cognos code.
- **`eval(source)`** — Runtime builtin that parses and executes Cognos source dynamically. Registers flows, runs main, executes bare statements.
- **`invoke(name, args)`** — Calls a dynamically registered flow by name.
- **`examples/meta-agent.cog`** — Reference implementation of the meta-agent pattern.

### Flow of Execution

```
meta-agent.cog
  │
  ├── Gather context (repo structure, file listing)
  ├── Load grammar (prompts/cognos-flow-generator.md)
  ├── think() → Master LLM generates Cognos flow
  ├── write_text() → Save generated flow for inspection
  ├── eval() → Parse and register the generated flow
  ├── invoke("solve", {}) → Execute it
  │     ├── read_file(), shell() → deterministic exploration
  │     ├── think() → LLM reasoning where needed
  │     ├── edit_file() → apply fixes
  │     └── git_diff() → capture result
  └── Output patch
```

### Error Recovery

If the generated flow fails (parse error, runtime error, wrong result), the meta-agent catches the error and falls back to a standard tool loop. Future iterations could re-prompt the master with the error to generate a corrected flow.

## What Makes This Different

| Aspect | Traditional Agent | Cognos Meta-Agent |
|--------|------------------|-------------------|
| State management | Conversation history (token cost) | Variables (zero cost) |
| Control flow | Emergent (hope the LLM loops) | Explicit (if/for/loop) |
| Debuggability | Read 15-turn conversation | Read generated source code |
| LLM calls | Every action needs a round-trip | Only where reasoning needed |
| Improvability | Tweak system prompts | Improve the grammar + master prompt |
| Composability | None | Flows call flows, eval within eval |
| Learning | None | Memory recalls successful patterns |

## Cognos's Unfair Advantage

This approach requires a language that LLMs can write reliably. Python has too many ways to do anything — LLM-generated Python is fragile. Cognos's restricted grammar (no classes, no exceptions beyond try/catch, no imports beyond `import`, no dynamic typing surprises) makes LLM generation reliable:

- **8 rules** cover the entire language
- **One way** to define functions (`flow`)
- **One way** to loop (`loop:` + `break`, or `for`)
- **Typed parameters** prevent argument confusion
- **No hidden state** — everything is explicit

The language was designed to be written by humans AND machines.
