# Cognos Language Specification

Version: 0.2.0

## 1. Introduction

Cognos is a statically-typed, imperative programming language designed for defining agentic workflows. The language provides deterministic control structures around non-deterministic computation (LLM calls), enabling reliable and testable AI-powered automation.

### 1.1 Design Principles

- **Explicitness**: Every LLM interaction is visible in the code. Input sources are declared, not assumed.
- **Type Safety**: Static type checking prevents runtime errors
- **Channel Agnosticism**: Flows declare *what* input they need, not *where* it comes from
- **Hybrid Determinism**: Deterministic control flow with explicit non-deterministic operations

## 2. Core Concepts

### 2.1 Flows

Flows are the fundamental unit of composition in Cognos, analogous to functions in other languages. **Input is received through flow parameters** — the runtime binds them from whatever channel invokes the flow (stdin, TUI, API, another flow).

```cognos
# No input needed
flow hello():
    emit("Hello, World!")

# Takes input — caller decides where it comes from
flow echo(input: String):
    emit(input)

# Multiple inputs with return type
flow greet(name: String, style: String) -> String:
    return think(name, system=style)
```

Key properties:
- Flows can take zero or more typed parameters
- **Parameters are the input contract** — the flow never cares about the channel
- Flows can optionally specify a return type
- The flow body consists of sequential statements
- Flows are first-class values and can be passed as arguments

### 2.2 Input Binding

When a flow is invoked, the runtime binds its parameters:

| Context | Binding |
|---------|---------|
| `cognos run file.cog` (CLI) | Parameters read from stdin |
| Neocognos TUI | Parameters bound from user message |
| API call | Parameters bound from request body |
| Flow-to-flow call | Parameters passed as arguments |

The flow itself is identical in all cases. This is the **channel agnosticism** principle.

### 2.3 Steps

Within a flow, execution proceeds through sequential steps (statements). Each step creates bindings or performs side effects.

### 2.4 Bindings

Bindings associate names with values:

```cognos
name = expression
```

### 2.5 Types

Cognos uses a static type system to ensure correctness.

#### Primitive Types
- `String`: Unicode string
- `Bool`: Boolean true/false
- `Int`: 64-bit signed integer
- `Float`: 64-bit floating point number

#### Container Types
- `List[T]`: Ordered collection of elements of type T
- `Map[K,V]`: Key-value mapping from type K to type V
- `Optional[T]`: Value that may or may not be present

#### Custom Types
```cognos
type TypeName = {
    field1: Type1,
    field2: Type2,
}
```

### 2.6 The LLM as Co-processor

The `think()` function is the only non-deterministic primitive in Cognos. It represents an LLM call and is the boundary between deterministic and non-deterministic computation.

```cognos
result = think(context, model="claude-sonnet-4-20250514", system="prompt", tools=[tool_list])
```

## 3. Built-in Functions

### 3.1 Output

#### `emit(value) -> Void`
Outputs a value from the flow to whatever channel is listening.
```cognos
emit("Hello, World!")
emit(response)
```

#### `pass`
No-op statement. Used for empty flow bodies.
```cognos
flow noop():
    pass
```

### 3.2 LLM Functions

#### `think(context, model="", system="", tools=[], output=Type) -> Type`
Invokes the LLM with the given context and constraints.

Parameters:
- `context`: The primary input to the LLM (any type that can be stringified)
- `model`: Optional model identifier (String). Can be a variable for reuse.
- `system`: Optional system prompt (String)
- `tools`: Optional list of available tools (List[String])
- `output`: Optional output type constraint for structured generation

```cognos
# Simple call
response = think(user_query)

# With model selection
fast = "claude-sonnet-4-20250514"
smart = "claude-opus-4-0520"

plan = think(task, model=fast, system="Plan only, don't implement.")
result = think(plan, model=smart, system="Implement the plan.", tools=[read_file, write_file])
```

#### `act(response) -> Any`
Executes tool calls from a `think()` response that contains tool calls.
```cognos
tool_result = act(llm_response)
```

### 3.3 Memory Functions

#### `remember(content) -> Void`
Stores content in long-term memory.
```cognos
remember("User prefers concise responses")
```

#### `recall(query) -> List[Fact]`
Searches long-term memory for relevant facts.
```cognos
facts = recall("user preferences")
```

### 3.4 System Functions

#### `run(command: String) -> RunResult`
Executes a shell command and returns the result.

```cognos
type RunResult = {
    success: Bool,
    output: String,
    error: String,
    exit_code: Int
}

result = run("cargo test")
if result.success:
    emit("Tests passed!")
```

#### `log(message: String) -> Void`
Outputs a debug message (not to the user channel).
```cognos
log("Processing step 3")
```

### 3.5 Flow Calls

Flows can call other flows directly. Parameters are passed by value, each flow gets its own scope:
```cognos
flow summarize(text: String) -> String:
    return think(text, model="claude-sonnet-4-20250514", system="Summarize concisely.")

flow classify(text: String) -> String:
    return think(text, model="qwen2.5:1.5b", system="Reply SIMPLE or COMPLEX only.")

flow main(input: String):
    complexity = classify(input)
    summary = summarize(input)
    emit(f"{complexity}: {summary}")
```

### 3.6 String Interpolation (F-strings)

F-strings allow embedding expressions inside string literals:
```cognos
name = "Cognos"
emit(f"Hello, {name}!")              # → Hello, Cognos!
emit(f"{1 + 2} items")              # → 3 items
emit(f"{name.length} chars")        # → 6 chars
emit(f"{name} has {name.length} characters")  # → Cognos has 6 characters
```

Any valid Cognos expression can appear inside `{}`.

## 4. Control Flow

### 4.1 Conditional Execution

```cognos
if condition:
    # statements
elif other_condition:
    # statements
else:
    # statements
```

### 4.2 Bounded Loops

```cognos
loop max=N:
    # statements
    if condition:
        break
    if other_condition:
        continue
```

The `max` parameter prevents infinite loops and is required.

### 4.3 Iteration

```cognos
for item in collection:
    # statements with access to 'item'
```

### 4.4 Parallel Execution

```cognos
result1, result2, result3 = parallel:
    expensive_operation_1()
    expensive_operation_2()
    expensive_operation_3()
```

### 4.5 Error Handling

```cognos
try:
    result = might_fail()
catch error:
    log("Failed: " + error.message)
    result = default_value
```

## 5. Type System

### 5.1 Type Inference

Types are inferred where unambiguous:
```cognos
name = "hello"  # inferred as String
count = 42      # inferred as Int
```

### 5.2 Structured Output Constraints

```cognos
type Analysis = {
    sentiment: String,
    confidence: Float,
    key_topics: List[String]
}

result = think(text, system="Analyze this text", output=Analysis)
# result.sentiment, result.confidence, etc. are type-safe
```

## 6. Operators

### 6.1 Context Concatenation

The `+` operator concatenates context for LLM calls:
```cognos
context = user_input + previous_response + additional_info
response = think(context)
```

### 6.2 Field Access

```cognos
if response.has_tool_calls:
    act(response)
```

### 6.3 Arithmetic Operators
`+` (add/concat), `-`, `*`, `/`

### 6.4 Comparison Operators
`==`, `!=`, `<`, `>`, `<=`, `>=`

### 6.5 Logical Operators
`and`, `or`, `not`

## 7. Comments

```cognos
# This is a comment
result = think(input)  # End-of-line comment
```

## 8. Complete Example

A multi-model agentic assistant with tool use:

```cognos
flow assistant(input: String):
    fast = "claude-sonnet-4-20250514"
    smart = "claude-opus-4-0520"

    # Classify complexity
    classification = think(input, model=fast, system="Reply SIMPLE or COMPLEX only.")

    # Pick model based on complexity
    if classification == "COMPLEX":
        model = smart
    else:
        model = fast

    # Agentic loop
    response = input
    loop max=30:
        response = think(response, model=model, system="Be helpful.", tools=[read_file, write_file, run])
        if response.has_tool_calls:
            result = act(response)
            response = result
        else:
            break

    emit(response)
```

## 9. Grammar (PEG)

```peg
Program <- Flow*

Flow <- "flow" Identifier "(" ParameterList? ")" ("->" Type)? ":" NEWLINE INDENT Statement* DEDENT

ParameterList <- Parameter ("," Parameter)*
Parameter <- Identifier ":" Type

Type <- Identifier ("[" Type ("," Type)* "]")?

Statement <- Assignment / EmitStatement / ReturnStatement / IfStatement / 
             LoopStatement / ForStatement / TryStatement / ParallelStatement /
             BreakStatement / ContinueStatement / PassStatement / ExprStatement

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
Unary <- "not" Unary / Postfix
Postfix <- Primary (("." Identifier) / ("(" ArgumentList? ")"))*
Primary <- Identifier / FStringLiteral / StringLiteral / IntLiteral / FloatLiteral / BoolLiteral / ListLiteral / MapLiteral / "(" Expression ")"

ArgumentList <- Argument ("," Argument)*
Argument <- (Identifier "=")? Expression

IfStatement <- "if" Expression ":" Block ("elif" Expression ":" Block)* ("else" ":" Block)?
LoopStatement <- "loop" ("max=" IntLiteral)? ":" Block
ForStatement <- "for" Identifier "in" Expression ":" Block
TryStatement <- "try" ":" Block "catch" Identifier ":" Block
ParallelStatement <- Identifier ("," Identifier)* "=" "parallel" ":" Block

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

## 10. Compilation

Cognos programs compile to Neocognos kernel `StageDef`s. The compiler maps:

| Cognos | Kernel Stage |
|--------|-------------|
| `think()` | Think stage |
| `act()` | Act stage |
| `emit()` | Emit stage |
| `if/elif/else` | Conditional stage |
| `loop` | Loop stage |
| `run()` | Tool call stage |
| Flow params | Receive stage (bound by runtime) |

The kernel doesn't change — Cognos is a better syntax for the same execution model.

## 11. Execution Modes

| Mode | Command | Description |
|------|---------|-------------|
| Interpret | `cognos run file.cog` | Tree-walking interpreter, stdin/stdout |
| Parse | `cognos parse file.cog` | Pretty-print parsed AST |
| REPL | `cognos repl` | Interactive read-eval-print loop |
| Tokens | `cognos tokens file.cog` | Raw token dump |
