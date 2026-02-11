# Cognos Language Specification

Version: 0.4.0

## 1. Introduction

Cognos is a programming language for agentic workflows. It provides deterministic control structures around non-deterministic computation (LLM calls). The LLM is a co-processor — you call it via `think()`, but everything else is explicit, typed, and testable.

### 1.1 Design Principles

- **Start with nothing, add what you need.** No accidental complexity.
- **Channel agnostic.** Flows declare *what* input they need, not *where* it comes from.
- **The LLM is a co-processor.** `think()` is the only non-deterministic primitive.
- **Sandboxed by design.** Only builtins can interact with the outside world.
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
| `None` | Returned by `emit()`, `log()`. No literal. |

### 2.4 Truthiness

| Falsy | Truthy |
|-------|--------|
| `false`, `0`, `0.0`, `""`, `[]`, `{}`, `None` | Everything else |

## 3. Flows

Flows are the fundamental unit of composition. Input is received through parameters — the runtime binds them from whatever channel invokes the flow.

```cognos
flow main():
    emit("Hello, World!")

flow greet(name: String) -> String:
    return f"Hello, {name}!"

flow assistant(input: String):
    response = think(input, model="qwen2.5:7b")
    emit(response)
```

### 3.1 Input Binding

| Context | How params are bound |
|---------|---------------------|
| `cognos run file.cog` | Read from stdin |
| Neocognos TUI | Bound from user message |
| API call | Bound from request body |
| Flow-to-flow call | Passed as arguments |

### 3.2 Flow Composition

Flows call other flows directly. Each gets its own scope:

```cognos
flow add(a: Int, b: Int) -> Int:
    return a + b

flow main():
    emit(add(2, 3))    # → 5
```

## 4. Built-in Functions

### 4.1 LLM

#### `think(context, model="", system="", tools=[]) -> String`
Invokes the LLM.

```cognos
response = think(input)
response = think(input, model="qwen2.5:7b", system="Be concise.")
```

#### `act(response) -> Any`
Executes tool calls from a `think()` response. *(Stubbed in interpreter mode.)*

### 4.2 Output

#### `emit(value)`
Outputs a value to the channel.

#### `log(message)`
Outputs to stderr (debug, not user-visible).

### 4.3 System

#### `run(command: String) -> String`
Executes a shell command, returns stdout.

```cognos
result = run("cargo test")
emit(result)
```

## 5. Operators

### 5.1 Arithmetic
`+` (add/concat), `-`, `*`, `/`, unary `-`

### 5.2 Comparison
`==`, `!=`, `<`, `>`, `<=`, `>=`

### 5.3 Logical
`and`, `or`, `not`

### 5.4 Indexing
```cognos
items[0]        # list index
items[-1]       # negative = from end
"hello"[0]      # string index → "h"
map["key"]      # map lookup
```

### 5.5 Field Access
```cognos
obj.field       # map field, or .length on String/List/Map
```

## 6. Methods

### 6.1 String Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.upper()` | String | `"hi".upper()` → `"HI"` |
| `.lower()` | String | `"HI".lower()` → `"hi"` |
| `.strip()` | String | `"  hi  ".strip()` → `"hi"` |
| `.contains(s)` | Bool | `"hello".contains("ell")` → `true` |
| `.starts_with(s)` | Bool | `"hello".starts_with("he")` → `true` |
| `.ends_with(s)` | Bool | `"hello".ends_with("lo")` → `true` |
| `.replace(from, to)` | String | `"hello".replace("l", "L")` → `"heLLo"` |
| `.split(delim)` | List[String] | `"a,b".split(",")` → `["a", "b"]` |
| `.length` | Int | `"hello".length` → `5` |

### 6.2 List Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.contains(val)` | Bool | `[1,2,3].contains(2)` → `true` |
| `.join(sep)` | String | `[1,2].join("-")` → `"1-2"` |
| `.reversed()` | List | `[1,2,3].reversed()` → `[3,2,1]` |
| `.length` | Int | `[1,2,3].length` → `3` |

### 6.3 Map Methods

| Method | Returns | Example |
|--------|---------|---------|
| `.keys()` | List[String] | `{"a":1}.keys()` → `["a"]` |
| `.values()` | List | `{"a":1}.values()` → `[1]` |
| `.contains(key)` | Bool | `{"a":1}.contains("a")` → `true` |
| `.length` | Int | `{"a":1}.length` → `1` |

## 7. Control Flow

### 7.1 Conditional

```cognos
if condition:
    body
elif other:
    body
else:
    body
```

### 7.2 Loops

```cognos
# Infinite loop — exits via break or return
loop:
    if done:
        break

# Bounded loop
loop max=10:
    response = think(response)
    if not response.has_tool_calls:
        break
```

### 7.3 For Loops

```cognos
for item in [1, 2, 3]:       # iterate list
    emit(item)

for ch in "hello":            # iterate characters
    emit(ch)

for key in {"a": 1, "b": 2}: # iterate map keys
    emit(key)
```

`break` and `continue` work in both `loop` and `for`.

## 8. String Interpolation

```cognos
name = "World"
emit(f"Hello, {name}!")           # → Hello, World!
emit(f"{1 + 2} items")           # → 3 items
emit(f"{name.length} chars")     # → 5 chars
```

Any valid expression can appear inside `{}`.

## 9. Comments

```cognos
# This is a comment
x = 42  # end-of-line comment
```

## 10. Grammar (PEG)

```peg
Program <- Flow*

Flow <- "flow" Identifier "(" ParameterList? ")" ("->" Type)? ":" NEWLINE INDENT Statement* DEDENT

ParameterList <- Parameter ("," Parameter)*
Parameter <- Identifier ":" Type
Type <- Identifier ("[" Type ("," Type)* "]")?

Statement <- Assignment / EmitStatement / ReturnStatement / IfStatement /
             LoopStatement / ForStatement / BreakStatement / ContinueStatement /
             PassStatement / ExprStatement

Assignment <- Identifier "=" Expression NEWLINE
EmitStatement <- "emit" "(" Expression ")" NEWLINE
ReturnStatement <- "return" Expression NEWLINE
PassStatement <- "pass" NEWLINE
BreakStatement <- "break" NEWLINE
ContinueStatement <- "continue" NEWLINE
ExprStatement <- Expression NEWLINE

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

## 11. CLI

```
cognos <file.cog>              # run a program
cognos run [-v|-vv|-vvv] <file> # run with logging verbosity
cognos parse <file.cog>         # pretty-print parsed AST
cognos tokens <file.cog>        # show raw tokens
cognos repl                     # interactive REPL
```

Logging: `-v` info, `-vv` debug, `-vvv` trace. Or set `COGNOS_LOG=info|debug|trace`.

## 12. Error System

Every token has a specific, context-aware error message with optional hints:

```
Error: unexpected '=' — not a valid expression
  hint: did you mean '==' for comparison?

Error: 'let' is not needed — just write: name = value

Error: cannot String + Int — String + Int not supported
```

The error system is exhaustive — adding a new token requires defining its error message.
