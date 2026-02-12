# Cognos Language Specification

Version: 0.5.0

## 1. Introduction

Cognos is a programming language for agentic workflows. It provides deterministic control structures around non-deterministic computation (LLM calls). The LLM is a co-processor — you call it via `think()`, but everything else is explicit, typed, and testable.

### 1.1 Design Principles

- **Start with nothing, add what you need.** No accidental complexity.
- **Channel agnostic.** Flows declare *what* input they need, not *where* it comes from.
- **The LLM is a co-processor.** `think()` is the only non-deterministic primitive.
- **Environment agnostic.** Same `.cog` runs in production or against a mock.
- **Sandboxed by design.** Only builtins can interact with the outside world.
- **Builtins are atomic.** Rust builtins perform one I/O operation. Orchestration, loops, and decision-making belong in `.cog` flows.
- **Behaviors in language, not runtime.** Agent behaviors (retry, tool loops, memory) are `.cog` flows, not Rust code.
- **One feature, multiple use cases.** Prefer one general mechanism over multiple specialized ones.
- **Testability first.** Every agent testable without LLM, network, or filesystem. Mocks are first-class.
- **Lean core runtime.** Cognos is a domain language for agents, not a general-purpose language. Think, act, observe, remember — everything else is out of scope.
- **Platform portable.** `.cog` files run anywhere the interpreter compiles.

## 2. Types

### 2.1 Primitive Types

| Type | Literal | Example |
|------|---------|---------|
| `String` | `"..."` or `f"...{expr}..."` | `"hello"`, `f"hi {name}"` |
| `Int` | digits | `42`, `-5` |
| `Float` | digits.digits | `3.14` |
| `Bool` | `true` / `false` | `true` |

### 2.2 Collection Types

| Type | Literal | Example |
|------|---------|---------|
| `List[T]` | `[a, b, c]` | `[1, 2, 3]` |
| `Map[K,V]` | `{"k": v}` | `{"name": "cognos"}` |

### 2.3 Special Types

| Type | Description |
|------|-------------|
| `None` | Returned by `write()`, `log()`, `save()`. No literal. |
| `Handle` | I/O endpoint: `stdin`, `stdout`, or `file("path")` |
| `Module` | Built-in module: `http` |

### 2.4 Custom Types

```cognos
type Review:
    score: Int
    summary: String
    tags: List
```

Used with `think(input, format="Review")` to get structured LLM output.

### 2.4.1 Generic Type Validation

List and Map types support inner type parameters for validation:

```cognos
type Insight:
    text: String
    score: Int

type Review:
    score: Int
    insights: List[Insight]    # each element validated as Insight
    tags: List                  # unvalidated (any list)
    metadata: Map[String, Int]  # values validated as Int
```

When `think(input, format="Review")` returns data, `List[Insight]` validates every element against the `Insight` type definition. `Map[String, Int]` validates all values are integers.

### 2.4.2 Optional Fields

Fields can be marked optional with `?` — they may be absent from LLM responses:

```cognos
type Config:
    name: String
    description?: String    # may be missing
    timeout?: Int           # may be missing
```

Optional fields are not included in the "required" set during validation. If present, they are still type-checked.

### 2.4.3 Enum Types

Enum types restrict a field to a fixed set of string values:

```cognos
type Severity: "low" | "medium" | "high" | "critical"

type Issue:
    title: String
    severity: Severity
```

Enum values are validated — if the LLM returns a value not in the set, validation fails.

### 2.5 Truthiness

| Falsy | Truthy |
|-------|--------|
| `false`, `0`, `0.0`, `""`, `[]`, `{}`, `None` | Everything else |

### 2.6 Type Coercion

Mixed `Int`/`Float` arithmetic auto-promotes: `1 + 2.5 → 3.5`

## 3. Imports

```cognos
import "path/to/module.cog"
```

- Must appear at the top of the file, before types and flows
- Paths resolve relative to the importing file's directory
- Recursive imports supported
- Imported flows and types are registered in the current scope
- Last import wins on name collisions

## 4. Flows

Flows are the fundamental unit of composition.

```cognos
flow main():
    write(stdout, "Hello, World!")

flow greet(name: String) -> String:
    return f"Hello, {name}!"

flow assistant(input: String):
    response = think(input, model="qwen2.5:7b")
    write(stdout, response)
```

### 4.1 Docstrings

The first string literal in a flow body is its description (used as tool description):

```cognos
flow search(query: String) -> String:
    "Search the web for information"
    return http.get(f"https://api.example.com/search?q={query}")
```

### 4.2 Input Binding

| Context | How params are bound |
|---------|---------------------|
| `cognos run file.cog` | Read from stdin |
| Neocognos TUI | Bound from user message |
| API call | Bound from request body |
| Flow-to-flow call | Passed as arguments |
| As tool | Bound from LLM tool call arguments |

### 4.3 Flow Composition

```cognos
flow add(a: Int, b: Int) -> Int:
    return a + b

flow main():
    write(stdout, add(2, 3))    # → 5
```

### 4.4 Flows as Tools

Flows can be passed to `think()` as tools. The interpreter auto-generates JSON schemas from flow signatures:

```cognos
import "lib/exec.cog"

flow shell(command: String) -> String:
    "Execute a sandboxed shell command"
    return __exec_shell__(command)

flow main():
    response = think("What time is it?",
        model="claude-sonnet-4-20250514",
        tools=["shell"])
    if response["has_tool_calls"]:
        response = exec(response, tools=["shell"])
    write(stdout, response)
```

## 5. Built-in Functions

### 5.1 LLM

#### `think(context, model="", system="", tools=[], format="") -> String | Map`

Invokes the LLM. The only non-deterministic primitive.

```cognos
# Simple call
response = think(input)

# With model and system prompt
response = think(input, model="claude-sonnet-4-20250514", system="Be concise.")

# With tools — returns Map with content + tool_calls
response = think(input, tools=["shell", "search"])

# With structured output — returns Map matching type schema
review = think(code, format="Review")
```

**Model routing:** `claude-*` → Claude CLI/API, anything else → Ollama.

#### `invoke(name, args) -> Value`

Calls a flow by string name with a Map of keyword arguments. This is the atomic primitive for dynamic dispatch.

```cognos
result = invoke("shell", {"command": "date"})
# equivalent to: result = shell(command="date")
```

> **Note:** `exec()` (tool call execution from `think()` responses) has moved to the standard library at `lib/exec.cog`. Import it with `import "lib/exec.cog"`. See [Standard Library](#standard-library) below.

### 5.2 I/O

#### `read(handle?) -> String`

Reads from a handle. Default: `stdin`.

```cognos
line = read(stdin)           # read one line from stdin
content = read(file("data.txt"))  # read entire file
```

#### `write(handle, content)`

Writes to a handle.

```cognos
write(stdout, "Hello!")           # print to stdout
write(file("out.txt"), content)   # write to file
```

#### `file(path) -> Handle`

Creates a file handle.

```cognos
write(file("output.txt"), "data")
content = read(file("input.txt"))
```

### 5.3 Persistence

#### `save(path, value)`

Serializes any Cognos value to a JSON file.

```cognos
save("state.json", {"history": history, "count": 42})
```

#### `load(path) -> Value`

Deserializes a JSON file back to a Cognos value.

```cognos
state = load("state.json")
```

### 5.4 Shell

#### `__exec_shell__(command) -> String`

Low-level shell primitive. Requires `--allow-shell` flag.

Typically wrapped in a user-defined flow for sandboxing:

```cognos
flow shell(command: String) -> String:
    "Execute a sandboxed shell command. Output limited to 50 lines."
    return __exec_shell__(f"{command} | head -50")
```

### 5.5 Logging

#### `log(message)`

Outputs to stderr (debug, not user-visible).

#### `print(value)`

Alias for `log()`.

### 5.6 Built-in Variables

| Variable | Type | Description |
|----------|------|-------------|
| `stdin` | Handle | Standard input handle |
| `stdout` | Handle | Standard output handle |
| `http` | Module | HTTP client |

## 6. Native Modules

### 6.1 `http`

| Function | Description |
|----------|-------------|
| `http.get(url)` | HTTP GET, returns body as String |
| `http.post(url, body)` | HTTP POST, returns body as String |

## 7. Operators

### 7.1 Arithmetic

`+` (add/concat lists/strings), `-`, `*`, `/`, unary `-`

`Int + Float` → `Float` (auto-promotion)

### 7.2 Comparison

`==`, `!=`, `<`, `>`, `<=`, `>=`

### 7.3 Logical

`and`, `or`, `not`

### 7.4 Indexing

```cognos
items[0]        # list index
items[-1]       # negative = from end
"hello"[0]      # string index → "h"
map["key"]      # map lookup
```

### 7.5 Field Access

```cognos
obj.field       # map field
s.length        # string/list/map length
```

## 8. Methods

### 8.1 String Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.upper()` | String | `"hi".upper()` → `"HI"` |
| `.lower()` | String | `"HI".lower()` → `"hi"` |
| `.strip()` | String | `"  hi  ".strip()` → `"hi"` |
| `.contains(s)` | Bool | `"hello".contains("ell")` → `true` |
| `.starts_with(s)` | Bool | `"hello".starts_with("he")` → `true` |
| `.ends_with(s)` | Bool | `"hello".ends_with("lo")` → `true` |
| `.replace(from, to)` | String | `"hello".replace("l", "L")` → `"heLLo"` |
| `.split(delim)` | List | `"a,b".split(",")` → `["a", "b"]` |
| `.truncate(max)` | String | `"hello".truncate(3)` → `"hel..."` |
| `.length` | Int | `"hello".length` → `5` |

### 8.2 List Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.contains(val)` | Bool | `[1,2,3].contains(2)` → `true` |
| `.join(sep)` | String | `[1,2].join("-")` → `"1-2"` |
| `.reversed()` | List | `[1,2,3].reversed()` → `[3,2,1]` |
| `.length` | Int | `[1,2,3].length` → `3` |

List concatenation: `[1, 2] + [3, 4]` → `[1, 2, 3, 4]`

### 8.3 Map Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.keys()` | List | `{"a":1}.keys()` → `["a"]` |
| `.values()` | List | `{"a":1}.values()` → `[1]` |
| `.contains(key)` | Bool | `{"a":1}.contains("a")` → `true` |
| `.length` | Int | `{"a":1}.length` → `1` |

## 9. Control Flow

### 9.1 Conditional

```cognos
if condition:
    body
elif other:
    body
else:
    body
```

### 9.2 Loops

```cognos
# Infinite loop — exits via break or return
loop:
    if done:
        break

# Bounded loop
loop max=10:
    response = think(response)
    if not response["has_tool_calls"]:
        break
```

### 9.3 For Loops

```cognos
for item in [1, 2, 3]:       # iterate list
    write(stdout, item)

for ch in "hello":            # iterate characters
    write(stdout, ch)

for key in {"a": 1, "b": 2}: # iterate map keys
    write(stdout, key)
```

`break` and `continue` work in both `loop` and `for`.

### 9.4 Try/Catch

```cognos
try:
    content = read(file("data.txt"))
catch err:
    content = "default"
    write(stdout, f"Warning: {err}")
```

- `err` variable is optional — omit it with just `catch:`
- Error message is bound as a String
- Variables set in the try block are visible after it (if no error)

### 9.5 Pass

```cognos
flow placeholder():
    pass
```

No-op statement for empty blocks.

## 10. String Interpolation

```cognos
name = "World"
write(stdout, f"Hello, {name}!")        # → Hello, World!
write(stdout, f"{1 + 2} items")        # → 3 items
write(stdout, f"{name.length} chars")  # → 5 chars
```

Any valid expression can appear inside `{}`.

## 11. Comments

```cognos
# This is a comment
x = 42  # end-of-line comment
```

## 12. Grammar (PEG)

```peg
Program <- Import* TypeDef* Flow*

Import <- "import" StringLiteral NEWLINE

TypeDef <- StructDef | EnumDef
StructDef <- "type" Identifier ":" NEWLINE INDENT TypeField* DEDENT
EnumDef <- "type" Identifier ":" StringLit ("|" StringLit)*
TypeField <- Identifier "?"? ":" Type NEWLINE

Flow <- "flow" Identifier "(" ParameterList? ")" ("->" Type)? ":" NEWLINE INDENT Statement* DEDENT

ParameterList <- Parameter ("," Parameter)*
Parameter <- Identifier ":" Type
Type <- Identifier ("[" Type ("," Type)* "]")?

Statement <- Assignment / ReturnStatement / IfStatement /
             LoopStatement / ForStatement / TryCatchStatement /
             BreakStatement / ContinueStatement /
             PassStatement / ExprStatement

Assignment <- Identifier "=" Expression NEWLINE
ReturnStatement <- "return" Expression NEWLINE
PassStatement <- "pass" NEWLINE
BreakStatement <- "break" NEWLINE
ContinueStatement <- "continue" NEWLINE
ExprStatement <- Expression NEWLINE

TryCatchStatement <- "try" ":" Block "catch" Identifier? ":" Block

Expression <- OrExpr
OrExpr <- AndExpr ("or" AndExpr)*
AndExpr <- Comparison ("and" Comparison)*
Comparison <- Addition (CompOp Addition)*
CompOp <- "==" / "!=" / "<" / ">" / "<=" / ">="
Addition <- Multiplication (("+" / "-") Multiplication)*
Multiplication <- Unary (("*" / "/") Unary)*
Unary <- "not" Unary / "-" Unary / Postfix
Postfix <- Primary (("." Identifier ("(" ArgList? ")")?) / ("[" Expression "]") / ("(" ArgList? ")"))*
Primary <- Identifier / FStringLiteral / StringLiteral / IntLiteral / FloatLiteral /
           BoolLiteral / ListLiteral / MapLiteral / "(" Expression ")"

ArgList <- Argument ("," Argument)*
Argument <- (Identifier "=")? Expression

IfStatement <- "if" Expression ":" Block ("elif" Expression ":" Block)* ("else" ":" Block)?
LoopStatement <- "loop" ("max=" IntLiteral)? ":" Block
ForStatement <- "for" Identifier "in" Expression ":" Block

Block <- NEWLINE INDENT Statement* DEDENT

Identifier <- [a-zA-Z_] [a-zA-Z0-9_]*
StringLiteral <- '"' [^"]* '"'
FStringLiteral <- 'f"' (FStringChar / '{' Expression '}')* '"'
IntLiteral <- [0-9]+
FloatLiteral <- [0-9]+ "." [0-9]+
BoolLiteral <- "true" / "false"
ListLiteral <- "[" (Expression ("," Expression)*)? "]"
MapLiteral <- "{" (StringLiteral ":" Expression ("," StringLiteral ":" Expression)*)? "}"
```

## 13. CLI

```
cognos run [flags] <file.cog>           # run a program
cognos test <file.cog> --env <mock>     # test with mock environment
cognos parse <file.cog>                 # pretty-print parsed AST
cognos tokens <file.cog>               # show raw tokens
cognos repl                            # interactive REPL
```

### Flags

| Flag | Description |
|------|-------------|
| `--allow-shell` | Enable `__exec_shell__()` |
| `--trace <path>` | Write JSONL trace events to file |
| `--trace-level metrics\|full` | Trace detail (default: metrics) |
| `--env <mock.json>` | Mock environment (for `cognos test`) |
| `-v` / `-vv` / `-vvv` | Log verbosity |

Env var: `COGNOS_LOG=info|debug|trace`

## 14. Error System

Every token has a specific, context-aware error message with optional hints:

```
Error: unexpected '=' — not a valid expression
  hint: did you mean '==' for comparison?

Error: 'let' is not needed — just write: name = value

Error: cannot String + Int — String + Int not supported
```

## 15. Environments

All I/O is routed through an `Env` trait. The interpreter never calls OS functions directly.

| Environment | Usage |
|-------------|-------|
| `RealEnv` | Default — real stdin/stdout, files, shell, LLM, HTTP |
| `MockEnv` | `cognos test --env mock.json` — canned responses, no network |

See [Environments docs](../docs/environments.md) for mock file format.

## 16. Concurrency

### `parallel:` Blocks

Run multiple statements concurrently. All branches execute in parallel threads and the block waits for all to complete before continuing.

```cognos
parallel:
    branch:
        code = read(file("src/main.rs"))
        review = think(code, model="claude-sonnet-4-20250514")
    branch:
        tests = shell("cargo test 2>&1")
        analysis = think(tests, system="Analyze")
    branch:
        metrics = shell("tokei src/")
# review, analysis, metrics all available here
```

Each `branch:` is an indented block of N statements. All branches run concurrently.

**Semantics:**
- Each `branch:` block runs in its own OS thread
- Block waits for ALL branches to complete
- Variables assigned inside branches are available after the parallel block
- No shared mutable state between branches — each gets a snapshot of current vars
- Errors in any branch propagate after all branches finish

### `async` / `await`

Fire-and-forget with later collection. `async` starts an expression in a background thread and returns a future handle. `await(handle)` blocks until the result is ready.

```cognos
handle = async deep_research("quantum computing")
# do other work while deep_research runs...
quick = think("what's 2+2?")
# now collect the result
result = await(handle)
```

**Semantics:**
- `async expr` spawns `expr` evaluation in a background thread, returns a `Future` handle
- `await(handle)` blocks until the async operation completes, returns the result
- The handle is a `Future` value — can be stored in variables, passed around
- Each `await` consumes the handle — awaiting the same handle twice is an error
- The background thread gets a snapshot of current variables and environment

### `select:` Blocks

Wait for the first event from multiple branches. Only one branch executes — whichever completes first.

```cognos
select:
    branch:
        input = read(stdin)
        write(stdout, f"Got: {input}")
    branch:
        result = await(handle)
        write(stdout, f"Task done: {result}")
```

**Semantics:**
- Each `branch:` runs concurrently in its own thread
- As soon as ONE branch completes all its statements, that branch wins
- All other branches are cancelled/abandoned
- Variables assigned in the winning branch are available after the select block
- This is analogous to Go's `select {}` or tokio's `select!`

### `cancel(handle)` Builtin

Cancel an async task by its future handle.

```cognos
handle = async do_task("something")
cancel(handle)
```

- Sets a cancellation flag on the async task
- The task will stop at the next statement boundary
- `await()` on a cancelled handle raises an error

### `remove(map, key)` Builtin

Remove a key from a map and return a new map without that key.

```cognos
tasks = {"a": 1, "b": 2}
tasks = remove(tasks, "a")
# tasks is now {"b": 2}
```

- Non-mutating — returns a new map (Cognos values are immutable)
- Removing a non-existent key returns the map unchanged

### Map Key Assignment

Assign a value to a map key using index syntax:

```cognos
tasks = {}
tasks["name"] = 42
tasks[variable_key] = value
```

- Creates or updates the key in the map
- Desugars to an internal `__map_set__` call that returns a new map
