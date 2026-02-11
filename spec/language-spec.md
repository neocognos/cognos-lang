# Cognos Language Specification

Version: 0.1.0

## 1. Introduction

Cognos is a statically-typed, imperative programming language designed for defining agentic workflows. The language provides deterministic control structures around non-deterministic computation (LLM calls), enabling reliable and testable AI-powered automation.

### 1.1 Design Principles

- **Explicitness**: Every LLM interaction is visible in the code
- **Type Safety**: Static type checking prevents runtime errors
- **Immutability by Default**: Bindings are immutable unless explicitly marked mutable
- **Hybrid Determinism**: Deterministic control flow with explicit non-deterministic operations

## 2. Core Concepts

### 2.1 Flows

Flows are the fundamental unit of composition in Cognos, analogous to functions in other languages.

```cognos
flow flow_name(param1: Type1, param2: Type2) -> ReturnType:
    # flow body
    return value
```

Key properties:
- Flows can take zero or more typed parameters
- Flows can optionally specify a return type
- The flow body consists of sequential statements
- Flows are first-class values and can be passed as arguments

### 2.2 Steps

Within a flow, execution proceeds through sequential steps (statements). Each step creates bindings or performs side effects.

### 2.3 Bindings

Bindings associate names with values and are immutable by default:

```cognos
name = expression
```

For mutable bindings (rarely needed):

```cognos
var name = expression
name = new_value  # reassignment
```

### 2.4 Types

Cognos uses a static type system to ensure correctness.

#### Primitive Types
- `Text`: Unicode string
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
    # ...
}
```

### 2.5 The LLM as Co-processor

The `think()` function is the only non-deterministic primitive in Cognos. It represents an LLM call and is the boundary between deterministic and non-deterministic computation.

```cognos
result = think(context, system="system prompt", tools=[tool_list], output=OutputType)
```

## 3. Built-in Functions

### 3.1 Input/Output Functions

#### `receive(Type) -> Type`
Receives input from the runtime environment (user, API, another agent).
```cognos
input = receive(Text)
```

#### `emit(value) -> Void`
Outputs a value from the flow.
```cognos
emit("Hello, World!")
```

### 3.2 LLM Functions

#### `think(context, system="", tools=[], output=Type) -> Type`
Invokes the LLM with the given context and constraints.

Parameters:
- `context`: The primary input to the LLM (any type that can be stringified)
- `system`: Optional system prompt (Text)
- `tools`: Optional list of available tools (List[Text])
- `output`: Optional output type constraint for structured generation

```cognos
response = think(user_query, 
    system="You are a helpful assistant",
    tools=[read_file, write_file],
    output=Text)
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

### 3.4 File System Functions

#### `read_file(path: Text) -> Text`
Reads the contents of a file.
```cognos
content = read_file("config.txt")
```

#### `write_file(path: Text, content: Text) -> Void`
Writes content to a file.
```cognos
write_file("output.txt", result)
```

### 3.5 System Functions

#### `run(command: Text) -> RunResult`
Executes a shell command and returns the result.

`RunResult` type:
```cognos
type RunResult = {
    success: Bool,
    output: Text,
    error: Text,
    exit_code: Int
}
```

#### `log(message: Text) -> Void`
Outputs a debug message.
```cognos
log("Processing step 3")
```

### 3.6 Flow Functions

#### `call flow_name(args...)`
Explicitly calls another flow. Can also use direct syntax: `flow_name(args)`.

```cognos
result = call process_data(input)
# Or simply:
result = process_data(input)
```

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

The `parallel` block executes all contained expressions concurrently and returns a tuple of results.

### 4.5 Error Handling

```cognos
try:
    # risky operations
    result = might_fail()
catch error:
    # handle error
    log("Operation failed: " + error.message)
    result = default_value
```

## 5. Type System

### 5.1 Type Inference

Types are inferred where unambiguous:
```cognos
name = "hello"  # inferred as Text
count = 42      # inferred as Int
```

### 5.2 Explicit Typing

Types can be explicitly declared:
```cognos
name: Text = "hello"
items: List[Int] = [1, 2, 3]
```

### 5.3 Structured Output Constraints

The `think()` function supports output type constraints for structured generation:

```cognos
type Analysis = {
    sentiment: Text,
    confidence: Float,
    key_topics: List[Text]
}

result = think(text, 
    system="Analyze this text",
    output=Analysis)
```

## 6. Data Flow and Operators

### 6.1 Context Concatenation

The `+` operator concatenates context for LLM calls:
```cognos
context = user_input + previous_response + additional_info
response = think(context)
```

### 6.2 Field Access

Use `.` to access fields of structured data:
```cognos
if response.has_tool_calls:
    act(response)

success_rate = results.map(r => r.success).count(true) / results.length
```

### 6.3 Comparison Operators

- `==`, `!=`: Equality and inequality
- `<`, `>`, `<=`, `>=`: Numeric/string comparison

### 6.4 Logical Operators

- `and`, `or`: Boolean logic
- `not`: Boolean negation

### 6.5 Lambda Expressions

Use `=>` for lambda expressions in functional operations:
```cognos
critical_issues = issues.filter(i => i.severity == "critical")
names = people.map(p => p.name)
total = numbers.reduce(0, (acc, n) => acc + n)
```

## 7. Comments

Line comments use `#`:
```cognos
# This is a comment
result = think(input)  # End-of-line comment
```

## 8. Grammar

The following PEG grammar defines Cognos syntax:

```peg
Program <- Flow*

Flow <- "flow" Identifier FunctionSignature ":" NEWLINE INDENT Statement* DEDENT

FunctionSignature <- "(" ParameterList? ")" ("->" Type)?

ParameterList <- Parameter ("," Parameter)*
Parameter <- Identifier ":" Type

Type <- PrimitiveType / ContainerType / CustomType
PrimitiveType <- "Text" / "Bool" / "Int" / "Float" / "Void"
ContainerType <- "List[" Type "]" / "Map[" Type "," Type "]" / "Optional[" Type "]"
CustomType <- Identifier

Statement <- Assignment / Expression / IfStatement / LoopStatement / ForStatement / 
             TryStatement / ParallelStatement / ReturnStatement / BreakStatement / 
             ContinueStatement

Assignment <- ("var")? Identifier (":" Type)? "=" Expression NEWLINE

Expression <- FunctionCall / BinaryOp / UnaryOp / FieldAccess / Identifier / Literal

FunctionCall <- Identifier "(" ArgumentList? ")"
ArgumentList <- Expression ("," Expression)*

BinaryOp <- Expression BinaryOperator Expression
BinaryOperator <- "+" / "==" / "!=" / "<" / ">" / "<=" / ">=" / "and" / "or"

UnaryOp <- UnaryOperator Expression
UnaryOperator <- "not" / "-"

FieldAccess <- Expression "." Identifier

IfStatement <- "if" Expression ":" Block ("elif" Expression ":" Block)* ("else" ":" Block)?

LoopStatement <- "loop" "max=" Int ":" Block

ForStatement <- "for" Identifier "in" Expression ":" Block

TryStatement <- "try" ":" Block "catch" Identifier ":" Block

ParallelStatement <- (Identifier ("," Identifier)*)? "=" "parallel" ":" Block

ReturnStatement <- "return" Expression? NEWLINE

BreakStatement <- "break" NEWLINE
ContinueStatement <- "continue" NEWLINE

Block <- NEWLINE INDENT Statement* DEDENT

Literal <- StringLiteral / IntLiteral / FloatLiteral / BoolLiteral / ListLiteral / MapLiteral

StringLiteral <- "\"" [^"]* "\""
IntLiteral <- [0-9]+
FloatLiteral <- [0-9]+ "." [0-9]+
BoolLiteral <- "true" / "false"
ListLiteral <- "[" (Expression ("," Expression)*)? "]"
MapLiteral <- "{" (MapEntry ("," MapEntry)*)? "}"
MapEntry <- Expression ":" Expression

Identifier <- [a-zA-Z_] [a-zA-Z0-9_]*

INDENT <- # Increased indentation
DEDENT <- # Decreased indentation
NEWLINE <- "\n" / "\r\n"
```

## 9. Semantics

### 9.1 Execution Model

1. Flows are compiled to an internal representation
2. The runtime maintains a stack of flow contexts
3. Bindings are immutable within their scope
4. The `think()` function is the only source of non-determinism
5. All other operations are deterministic and side-effect free (except I/O)

### 9.2 Type Checking

- All expressions must type-check at compile time
- `think()` calls with `output=Type` are checked for structural compatibility
- Flow parameters and return types are enforced

### 9.3 Memory Model

- Bindings are lexically scoped
- No global state except through explicit memory functions
- Garbage collection handles cleanup

This specification provides the foundation for implementing a Cognos compiler and runtime. The language balances expressiveness with safety, making AI-powered workflows as reliable as traditional code.