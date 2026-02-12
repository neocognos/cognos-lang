# Cognos Design Principles

## The Principles

### P1: Start with nothing, add what you need
No accidental complexity. Every feature must earn its place.

### P2: Channel agnostic
Flows declare *what* input they need, not *where* it comes from. `read(stdin)` works in CLI, TUI, API, Slack — the runtime maps the channel.

### P3: The LLM is a co-processor
`think()` is the only non-deterministic primitive. Everything else is explicit, typed, and testable.

### P4: Environment agnostic
Same `.cog` runs in production or against a mock. No code changes. The `Env` trait abstracts all I/O.

### P5: Sandboxed by design
Only builtins can interact with the outside world. Shell is disabled by default (`--allow-shell`). Sandbox policy lives in `.cog` files, not the runtime.

### P6: Builtins are atomic
Rust builtins perform exactly one I/O operation. No loops, no branching, no flow calls. Orchestration belongs in `.cog` flows.

### P7: Platform portable
`.cog` files run anywhere the interpreter compiles. No OS-specific constructs in the language.

### P8: Behaviors in language, not runtime
Agent behaviors (retry logic, tool loops, memory strategy) are defined in `.cog` files, not hardcoded in Rust. The runtime provides primitives; the programmer defines policy.

### P9: One feature, multiple use cases
Prefer a single general mechanism over multiple specialized ones. Example: `type` covers both structured output and JSON format validation.

### P10: Testability first
Every agent must be testable without LLM calls, network, or filesystem. Mock environments are a first-class concept, not an afterthought.

### P11: Lean core runtime
Cognos is a domain language for agents, not a general-purpose language. Resist adding features that exist in Python/JS/Go. If you need a hash map library, data science toolkit, or web framework — use a real language. Cognos handles the agent loop: think, act, observe, remember. Everything else is out of scope.

---

## Architecture Review

### Rust Builtins Audit (P6: Builtins are atomic)

| Builtin | Atomic? | Verdict |
|---------|---------|---------|
| `think()` | ⚠️ **VIOLATION** | See below |
| `invoke(name, args)` | ✅ | One flow call, returns result |
| `read(handle)` | ✅ | One read operation |
| `write(handle, content)` | ✅ | One write operation |
| `file(path)` | ✅ | Creates a handle value (pure) |
| `__exec_shell__(cmd)` | ✅ | One shell command |
| `save(path, value)` | ✅ | One file write |
| `load(path)` | ✅ | One file read |
| `log(msg)` / `print(msg)` | ✅ | One stderr write |
| `emit(value)` | ✅ | Alias for write(stdout, ...) |

### P6 Violations

#### `think()` — format validation + schema injection

`think()` currently does three things:
1. **Call the LLM** — atomic ✅
2. **Inject type schema into system prompt** (`format=`) — preparation, not I/O ⚠️
3. **Parse + validate JSON response** (`format=`) — post-processing ⚠️

Steps 2 and 3 are orchestration logic embedded in the builtin. They should be in `.cog`:

**Option A: Keep as-is (pragmatic)**
- Schema injection is prompt construction, not a separate I/O operation
- Validation is a check on the return value, not a loop or branch
- Moving it to `.cog` would require exposing `type_schema()` as a builtin

**Option B: Split (pure)**
- `think()` only calls LLM, returns raw string
- `type_schema(name)` builtin returns the JSON schema string
- `validate(value, type_name)` builtin checks a value against a type
- `formatted_think()` in `.cog` stdlib composes them

**Recommendation:** Option A for now. The format/validation logic doesn't loop, doesn't call flows, and doesn't do I/O. It's data transformation on the builtin's return value. A strict reading of P6 says split it, but the pragmatic cost is high (3 new builtins, user complexity).

**Decision needed from Reza.**

#### `think()` — tools= kwarg

`think()` builds tool JSON schemas from flow signatures when `tools=` is passed. This is:
- Reading flow definitions (data access, not I/O)
- Building JSON (data transformation)
- Passing to LLM call (part of the one I/O operation)

**Verdict:** Acceptable. It's preparing the LLM request, not orchestrating.

### Channel Agnosticism Audit (P2)

| Item | Status |
|------|--------|
| `read(stdin)` / `write(stdout, ...)` | ✅ Handle-based, channel-mapped |
| Flow params bound from stdin in CLI | ✅ |
| `print("> ")` for prompts | ⚠️ Leaks presentation into logic |
| `emit()` | ✅ Sugar for `write(stdout, ...)` |

**Minor violation:** When `main()` has params, the interpreter prints `> ` before reading each one. This presentation detail should be the runtime's concern, not injected by the interpreter. Low priority.

### Environment Agnosticism Audit (P4)

| I/O Operation | Routes through Env? |
|---------------|-------------------|
| stdin read | ✅ `env.read_stdin()` |
| stdout write | ✅ `env.write_stdout()` |
| file read | ✅ `env.read_file()` |
| file write | ✅ `env.write_file()` |
| shell exec | ✅ `env.exec_shell()` |
| LLM calls | ✅ `env.call_llm()` (mock) / direct (real) |
| HTTP | ✅ `env.http_get()` / `env.http_post()` |
| `save()` | ✅ `env.write_file()` |
| `load()` | ✅ `env.read_file()` |

**Violations:** `save()` and `load()` bypass the Env trait. In MockEnv, `save()` writes to real filesystem and `load()` reads from real filesystem. They should go through `env.write_file()` and `env.read_file()`.

### Sandbox Audit (P5)

| Item | Status |
|------|--------|
| Shell disabled by default | ✅ `--allow-shell` required |
| Shell policy in `.cog` | ✅ `shell()` is a user flow |
| File access unrestricted | ⚠️ No path restrictions |
| HTTP unrestricted | ⚠️ Any URL accessible |
| `invoke()` unrestricted | ⚠️ Can call any flow by name |

**Gaps:** No working directory sandbox or path restrictions yet. On the roadmap.

### Testability Audit (P10)

| Item | Status |
|------|--------|
| Mock environment | ✅ `cognos test --env mock.json` |
| All I/O through Env | ⚠️ `save()`/`load()` bypass |
| Deterministic mock replay | ✅ Traces → mocks → replay |
| trace-to-mock CLI | ✅ `cognos trace-to-mock` |
| CI-friendly | ✅ Zero external deps for mock tests |

### Behaviors in Language Audit (P8)

| Behavior | In `.cog`? |
|----------|-----------|
| Tool execution loop | ✅ `lib/exec.cog` |
| Auto tool loop | ✅ `lib/agent.cog` |
| Retry with validation | ✅ `lib/validated_think.cog` |
| Shell sandboxing | ✅ User-defined `shell()` flow |
| Memory strategy | ✅ Planned for `.cog` |
| Format validation | ⚠️ In Rust (see think() discussion above) |

---

## Action Items

### Fixed
1. ~~**`save()`/`load()` must route through Env**~~ — ✅ Now routes through `env.write_file()` / `env.read_file()`
2. ~~**Prompt `> ` in param binding**~~ — ✅ Removed presentation from interpreter

### Decided
3. **`think()` format validation stays in Rust** — it's the marshalling layer (type boundary between LLM world and Cognos world), not orchestration. Schema injection, JSON parsing, and type validation are all part of the single atomic operation: "call LLM, return typed value." This extends naturally to multimodal: `think(audio_handle, format="Transcript")` — same boundary, different media.

### Deferred (on roadmap)
4. Path restrictions for file access (P5)
5. HTTP URL restrictions (P5)
