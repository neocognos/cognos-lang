# Cognos

**Deterministic control over non-deterministic computation.**

Cognos is a programming language for agentic workflows. The LLM is a co-processor — you call it via `think()`, but everything else is deterministic, explicit, and testable.

## Quick Start

```bash
# Build
PATH="$HOME/.cargo/bin:$PATH" cargo build

# Run a program
./target/debug/cognos run examples/hello.cog

# Interactive REPL
./target/debug/cognos repl
```

## Hello World

```cognos
flow main():
    emit("Hello, World!")
```

## A Real Program

```cognos
flow summarize(text: String) -> String:
    return think(text, model="qwen2.5:7b", system="Summarize in one sentence.")

flow main(input: String):
    summary = summarize(input)
    emit(f"Summary: {summary}")
```

## Features

| Feature | Example |
|---------|---------|
| **Types** | `String`, `Int`, `Float`, `Bool`, `List`, `Map` |
| **LLM calls** | `think(input, model="sonnet", system="Be concise.")` |
| **F-strings** | `f"Hello {name}, you have {count} items"` |
| **Flow composition** | `result = my_flow(arg1, arg2)` |
| **Control flow** | `if`, `elif`, `else`, `loop max=N`, `break`, `continue` |
| **Shell commands** | `result = run("cargo test")` |
| **Maps** | `config = {"name": "cognos", "version": "0.1"}` |
| **REPL** | `cognos repl` — interactive experimentation |

## Design Principles

- **Start with nothing, add what you need.** No accidental complexity.
- **Channel agnostic.** Flows declare *what* input they need, not *where* it comes from.
- **The LLM is a co-processor.** `think()` is the only non-deterministic primitive.
- **Sandboxed by design.** Only builtins can interact with the outside world.
- **Platform portable.** `.cog` files run anywhere the interpreter compiles (Linux, macOS, Windows, WASM).

## CLI

```
cognos <file.cog>              # run a program
cognos run [-v|-vv|-vvv] <file> # run with logging
cognos parse <file.cog>         # pretty-print parsed AST
cognos tokens <file.cog>        # show raw tokens
cognos repl                     # interactive REPL
```

Logging: `-v` info, `-vv` debug, `-vvv` trace. Or `COGNOS_LOG=debug`.

## Examples

```bash
./target/debug/cognos run examples/empty.cog         # no-op (like main(){} in C)
./target/debug/cognos run examples/hello.cog          # hello world
echo "hi" | ./target/debug/cognos run examples/echo.cog    # echo input
echo "hi" | ./target/debug/cognos run examples/chat.cog    # LLM chat (needs Ollama)
```

## Docs

- [Language Specification](./spec/language-spec.md)
- [Compilation Strategy](./spec/compilation.md)
- [Examples](./examples/)

## Testing

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo test
```

38 tests: lexer, parser, interpreter, integration, type errors.
