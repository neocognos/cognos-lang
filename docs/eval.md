# eval() — Dynamic Code Execution

`eval()` parses and executes Cognos source code at runtime, enabling dynamic flow generation, meta-programming, and LLM-generated agents.

## Syntax

```
eval(source)                    # Parse and execute Cognos source
eval(source, {"x": 42})        # Execute with injected variables
```

## Behavior

### Flow Definitions
When the source contains `flow` definitions, they are registered in the interpreter and can be called via `invoke()`:

```
eval("flow double(n: Int) -> Int:\n    return n * 2")
result = invoke("double", {"n": 5})   # → 10
```

### Bare Statements
When the source contains bare statements (no `flow` keyword), they execute in the caller's scope:

```
x = 10
eval("x = x + 5")
write(stdout, x)    # → 15
```

### Variable Injection
The optional second argument injects variables before execution:

```
eval("write(stdout, f\"Hello {name}\")", {"name": "World"})
# → Hello World
```

### Main Flow
If the source defines a `main` flow, it is automatically executed:

```
eval("flow main():\n    write(stdout, \"auto-run\")")
# → auto-run
```

## Return Value

- `eval()` returns `None` when registering flows
- `eval()` returns the flow's return value when a `main` flow is present
- Bare statements execute for side effects; `eval()` returns `None`

## Error Handling

Parse errors and runtime errors propagate normally:

```
try:
    eval("this is not valid cognos")
catch e:
    write(stdout, f"Failed: {e}")
```

## Scope

- **Registered flows** are globally visible after `eval()` — any code can call them
- **Bare statements** run in the caller's scope — they can read and modify the caller's variables
- **Injected variables** are set in the caller's scope before execution
- **Already-imported flows** (from `import` statements) are available inside eval'd code — do not re-import

## Limitations

- `import` statements inside eval'd code resolve paths relative to the working directory, not the original source file
- Flows registered via `eval()` persist for the lifetime of the interpreter
- No sandboxing — eval'd code has full access to all builtins including `shell()`, `think()`, and `remember()`
