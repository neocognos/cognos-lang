# Error Handling

## Try/Catch

Catch errors gracefully instead of crashing.

```cognos
try:
    content = read(file("data.txt"))
catch err:
    content = "default value"
    write(stdout, f"Warning: {err}")
```

### Syntax

```cognos
try:
    <body>
catch [error_var]:
    <handler>
```

- `error_var` is optional — if provided, the error message is bound to it as a String
- The catch block runs only if the try block errors
- Variables set in the try block are visible after it (if no error)

### Examples

**Graceful file loading:**
```cognos
try:
    config = load("config.json")
catch:
    config = {"port": 8080, "debug": false}
```

**LLM fallback:**
```cognos
try:
    response = think(prompt, model="claude-sonnet-4-20250514")
catch err:
    write(stdout, f"Claude failed: {err}, trying local model...")
    response = think(prompt, model="qwen2.5:7b")
```

**Session persistence pattern:**
```cognos
flow main():
    # Load previous session or start fresh
    try:
        history = load("session.json")
    catch:
        history = []

    # ... agent loop ...

    # Save on exit
    save("session.json", history)
```

## Save / Load

Persist any Cognos value as JSON.

```cognos
save(path, value)    # writes value as pretty JSON
value = load(path)   # reads JSON back to Cognos value
```

### Type Mapping

| Cognos | JSON |
|--------|------|
| String | string |
| Int | number |
| Float | number |
| Bool | boolean |
| None | null |
| List | array |
| Map | object |

### Example

```cognos
data = {"name": "agent", "turns": 42, "history": ["hello", "world"]}
save("state.json", data)

# Later...
loaded = load("state.json")
write(stdout, loaded["name"])     # "agent"
write(stdout, loaded["history"])  # ["hello", "world"]
```
