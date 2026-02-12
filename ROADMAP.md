# Cognos Roadmap

## Language Usability
- [x] **Kwargs in flow calls** — `my_flow(input, format="Review")` instead of positional-only
- [x] **Multi-line expressions** — implicit continuation inside `()`, `[]`, `{}`
- [x] **String `*` repeat** — `"=" * 50` for formatting
- [x] **Default parameter values** — `flow greet(name: String, greeting: String = "Hello"):`
- [ ] **Nested List types** — `List[Insight]` validation, not just `List`

## Agent Intelligence
- [x] **Automatic tool loop** — `think()` with `auto_exec=true` handles multi-step tool chains
- [x] **Conversation compaction** — `history()`, `clear_history()` builtins + `lib/compact.cog` stdlib flow
- [ ] **Streaming output** — token-by-token `think()` output for responsiveness
- [ ] **Parallel/async** — `parallel:` blocks and `async`/`await` for concurrent tool calls
- [ ] **Agent-to-agent calls** — one agent invokes another as a tool

## Type System
- [ ] **Generic type validation** — `List[Int]`, `Map[String, Float]` checked at runtime
- [ ] **Optional fields** — `field?: Type` for non-required fields in type definitions
- [ ] **Enum types** — `type Status: "active" | "inactive" | "pending"`
- [ ] **Union types** — `String | Int` for flexible parameters

## Runtime & Platform
- [x] **OpenAI provider** — `model="gpt-4o"` alongside Claude and Ollama (gpt-*, o1-*, o3-*)
- [ ] **Readline/rustyline** — arrow keys, history in REPL
- [ ] **`cargo install cognos`** — publish to crates.io
- [ ] **WASM target** — run .cog files in the browser
- [x] **trace-to-mock** — convert recorded traces to mock files automatically
- [ ] **Working directory sandbox** — restrict shell + file access to specific paths

## Memory & State
- [ ] **`remember()`/`recall()` builtins** — semantic memory backed by vector DB
- [ ] **Session auto-save** — periodic saves, not just on exit
- [ ] **State snapshots** — save/restore full interpreter state for debugging

## Done
- [x] Lexer, parser, interpreter, REPL
- [x] Types: String, Int, Float, Bool, List, Map, Handle, Module, None
- [x] Custom type definitions with `type Name: field: Type`
- [x] `think()` with model routing (Claude CLI/API, Ollama)
- [x] `format=Type` structured output with strict validation
- [x] Flows as tools — `think(input, tools=["shell"])`
- [x] `invoke()` builtin for dynamic flow dispatch
- [x] `exec()` moved to `.cog` stdlib (`lib/exec.cog`), `agent_think()` in `lib/agent.cog`
- [x] Handle-based I/O — `read(stdin)`, `write(stdout, ...)`, `file("path")`
- [x] Native modules — `math.*`, `http.*`
- [x] `import "file.cog"` with circular detection
- [x] `try/catch` error handling
- [x] `save()`/`load()` JSON persistence
- [x] `for key, value in map:` iteration
- [x] String/list slicing `s[0:5]`
- [x] `--session` state persistence
- [x] Environment abstraction — `Env` trait, `RealEnv`, `MockEnv`
- [x] `cognos test --env mock.json`
- [x] JSONL tracing with metrics/full levels
- [x] `validated_think` library flow with auto-retry
- [x] Bool/None comparison operators
- [x] 139 tests, comprehensive edge case coverage
- [x] Full docs: spec v0.5.0, tracing, environments, imports, error handling
