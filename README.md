# Cognos

**Deterministic control over non-deterministic computation.**

Cognos is a programming language for agentic workflows. The LLM is a co-processor — you call it via `think()`, but everything else is deterministic, explicit, and testable.

## Quick Start

```bash
# Build
PATH="$HOME/.cargo/bin:$PATH" cargo build

# Run a program
cognos run examples/hello.cog

# Run with shell access
cognos run --allow-shell examples/shell-agent.cog

# Test with mock environment (no LLM, no network)
cognos test examples/chat.cog --env examples/mocks/chat-test.json

# Interactive REPL
cognos repl
```

## Hello World

```cognos
flow main():
    write(stdout, "Hello, World!")
```

## A Chat Agent (12 lines)

```cognos
flow main():
    history = []
    write(stdout, "Chat ready. Type 'quit' to exit.")
    loop:
        input = read(stdin)
        if input == "quit":
            break
        history = history + [f"User: {input}"]
        context = history.join("\n")
        response = think(context, model="qwen2.5:7b", system="You are a helpful assistant.")
        history = history + [f"Assistant: {response}"]
        write(stdout, response)
```

## A Shell Agent with Tools

```cognos
import "lib/shell.cog"

flow main():
    write(stdout, "Agent ready. Type 'quit' to exit.")
    loop:
        input = read(stdin)
        if input == "quit":
            break
        response = think(input, model="claude-sonnet-4-20250514",
            system="You are a helpful assistant with shell access.",
            tools=["shell"])
        if response["has_tool_calls"]:
            response = exec(response, tools=["shell"])
        write(stdout, response)
```

## Features

| Feature | Example |
|---------|---------|
| **Types** | `String`, `Int`, `Float`, `Bool`, `List`, `Map`, `Handle`, `Module` |
| **Custom types** | `type Review: score: Int, summary: String` |
| **LLM calls** | `think(input, model="claude-sonnet-4-20250514", system="Be concise.")` |
| **Structured output** | `think(input, format="Review")` — LLM returns typed Map |
| **Tools** | `think(input, tools=["search", "shell"])` — flows as LLM tools |
| **F-strings** | `f"Hello {name}, you have {count} items"` |
| **I/O handles** | `read(stdin)`, `write(stdout, ...)`, `read(file("path"))` |
| **Shell** | `__exec_shell__("ls")` (requires `--allow-shell`) |
| **Imports** | `import "lib/utils.cog"` |
| **Error handling** | `try: ... catch err: ...` |
| **Persistence** | `save("state.json", data)`, `load("state.json")` |
| **Native modules** | `math.sin(x)`, `math.pi`, `http.get(url)` |
| **Mock testing** | `cognos test agent.cog --env mock.json` |
| **Tracing** | `cognos run --trace trace.jsonl --trace-level full agent.cog` |
| **Control flow** | `if`/`elif`/`else`, `loop`, `for`, `break`, `continue` |
| **REPL** | `cognos repl` — interactive experimentation |

## Design Principles

- **Start with nothing, add what you need.** No accidental complexity.
- **Channel agnostic.** `read(stdin)` / `write(stdout, ...)` — same code works in CLI, TUI, API, Slack.
- **The LLM is a co-processor.** `think()` is the only non-deterministic primitive.
- **Environment agnostic.** Same `.cog` runs in production or against a mock — no code changes.
- **Sandboxed by design.** Shell disabled by default. `--allow-shell` for explicit opt-in.
- **Platform portable.** `.cog` files run anywhere the interpreter compiles.

## CLI

```
cognos run [flags] <file.cog>       # run a program
cognos test <file.cog> --env <mock> # test with mock environment
cognos parse <file.cog>             # pretty-print parsed AST
cognos tokens <file.cog>            # show raw tokens
cognos repl                         # interactive REPL
```

### Flags

| Flag | Description |
|------|-------------|
| `--allow-shell` | Enable shell execution |
| `--trace <path>` | Write JSONL trace events to file |
| `--trace-level metrics\|full` | Trace detail level (default: metrics) |
| `--env <mock.json>` | Mock environment file (for `cognos test`) |
| `--session <path>` | Auto-save/load variables between runs |
| `-v` / `-vv` / `-vvv` | Log verbosity (info/debug/trace) |

## LLM Providers

| Model prefix | Provider | Auth |
|-------------|----------|------|
| `claude-*` | Claude CLI → Anthropic API fallback | Max subscription or `ANTHROPIC_API_KEY` |
| anything else | Ollama (local) | None needed |

## Examples

```bash
cognos run examples/hello.cog                        # hello world
cognos run examples/chat.cog                         # LLM chat (needs Ollama)
cognos run --allow-shell examples/shell-agent.cog    # shell agent (needs Claude)
cognos run examples/import-test.cog                  # multi-file import
cognos run examples/try-catch.cog                    # error handling
cognos run examples/session-save.cog                 # save/load persistence
cognos test examples/chat.cog --env examples/mocks/chat-test.json  # mock test
```

## Docs

- [Language Specification](./spec/language-spec.md)
- [Tracing & Diagnostics](./docs/tracing.md)
- [Environments & Testing](./docs/environments.md)
- [Import System](./docs/import.md)
- [Error Handling & Persistence](./docs/error-handling.md)
- [Memory Design](./docs/memory.md)
- [Compilation Strategy](./spec/compilation.md)
- [Examples](./examples/)

## Testing

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test
```

91 tests: lexer, parser, interpreter, integration, type errors, mock environments.
