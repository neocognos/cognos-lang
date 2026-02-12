# Cognos Roadmap

## Priorities
1. Robustness — what we have must be rock solid
2. Agent patterns — concurrency, memory, debugging
3. Polish — developer experience, publishing

---

## Next Up

### Concurrency
- [ ] **`parallel:` blocks** — run N statements concurrently, wait for all, no shared mutable state
- [ ] **`async`/`await`** — fire off a flow, do other work, collect result later
- [ ] _Future: inter-thread communication (channels) — post-1.0_

### Memory
- [ ] **`remember()`/`recall()` builtins** — semantic memory backed by vector DB
- [ ] **Session auto-save** — periodic saves during long runs, not just on exit

### Debugging
- [ ] **`--snapshot-dir`** — auto-snapshot variables + history after each think() call
- [ ] **`cargo install cognos`** — publish to crates.io

### Agent Orchestration
- [ ] **Agent-to-agent calls** — one agent invokes another as a tool

### Cleanup
- [x] **Remove `math` module** — P11 violation (agents don't need trig)

---

## Done

### Language Usability
- [x] Kwargs in flow calls
- [x] Multi-line expressions (implicit continuation inside delimiters)
- [x] String `*` repeat
- [x] Default parameter values

### Type System (marshalling layer)
- [x] Custom type definitions — `type Review: score: Int`
- [x] `format=Type` structured output with strict validation
- [x] Generic type validation — `List[Insight]`, `Map[String, Int]`
- [x] Optional fields — `field?: Type`
- [x] Enum types — `type Severity: "low" | "medium" | "high"`

### Agent Intelligence
- [x] Automatic tool loop — `lib/agent.cog` (`agent_think()`)
- [x] Conversation compaction — `history()`, `clear_history()`, `lib/compact.cog`
- [x] OpenAI provider — `gpt-*`, `o1-*`, `o3-*` model routing

### Core
- [x] Lexer, parser, interpreter, REPL
- [x] Types: String, Int, Float, Bool, List, Map, Handle, Module, None
- [x] `think()` with multi-provider routing (Claude CLI/API, Ollama, OpenAI)
- [x] Flows as tools — `think(input, tools=["shell"])`
- [x] `invoke()` builtin for dynamic flow dispatch
- [x] `exec()` in stdlib (`lib/exec.cog`)
- [x] Handle-based I/O — `read(stdin)`, `write(stdout, ...)`, `file("path")`
- [x] Native modules — `http.*`
- [x] `import "file.cog"` with circular detection
- [x] `try/catch` error handling
- [x] `save()`/`load()` JSON persistence (via Env)
- [x] `for key, value in map:` iteration
- [x] String/list slicing `s[0:5]`
- [x] `--session` state persistence
- [x] Environment abstraction — `Env` trait, `RealEnv`, `MockEnv`
- [x] `cognos test --env mock.json`
- [x] JSONL tracing with metrics/full levels
- [x] `validated_think` library flow with auto-retry
- [x] trace-to-mock CLI
- [x] 163 tests, zero unwrap() calls

### Architecture
- [x] Design principles P1–P11 documented
- [x] Full architecture review — zero violations
- [x] `exec()` refactored from Rust to `.cog` stdlib

---

## Cut (P11: Lean core runtime)
- ~~WASM target~~ — platform engineering, not agent capability
- ~~Readline/rustyline~~ — REPL polish, not agent capability
- ~~Union types~~ — general-purpose type system creep
- ~~`math` module~~ — removed, agents don't need trig
