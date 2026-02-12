# Environments

Cognos agents are environment-agnostic. The same `.cog` file runs in production or against a mock — no code changes.

## How It Works

Every I/O operation goes through an `Env` trait:

| Operation | Method |
|-----------|--------|
| `read(stdin)` | `env.read_stdin()` |
| `write(stdout, ...)` | `env.write_stdout()` |
| `read(file("..."))` | `env.read_file()` |
| `write(file("..."), ...)` | `env.write_file()` |
| `__exec_shell__(...)` | `env.exec_shell()` |
| `think(...)` | `env.call_llm()` |
| `http.get(...)` | `env.http_get()` |
| `http.post(...)` | `env.http_post()` |

## Real Environment (default)

```bash
cognos run agent.cog
```

Uses real stdin/stdout, filesystem, shell, and LLM providers (Ollama/Claude).

## Mock Environment

```bash
cognos test agent.cog --env mock.json
```

All I/O is mocked. No network, no filesystem, no LLM calls. Instant, deterministic, free.

### Mock File Format

```json
{
  "stdin": ["user input line 1", "user input line 2", "quit"],
  "llm_responses": [
    "Simple text response",
    {
      "content": "",
      "tool_calls": [{"name": "shell", "arguments": {"command": "date"}}]
    },
    "Follow-up response after tool call"
  ],
  "shell": {
    "date": "Thu Feb 12 00:34:00 CET 2026",
    "ls -la": "total 42\n-rw-r--r-- 1 user user 100 file.txt"
  },
  "files": {
    "config.txt": "key=value",
    "data.json": "{\"items\": [1, 2, 3]}"
  },
  "allow_shell": true
}
```

### Fields

| Field | Description |
|-------|-------------|
| `stdin` | Array of strings — each `read(stdin)` consumes one |
| `llm_responses` | Array — each `think()` consumes one. String or object with `content` + `tool_calls` |
| `shell` | Map of command → output. Exact match or base command (before `\|`) |
| `files` | Map of path → content for `read(file(...))` |
| `allow_shell` | Whether shell execution is allowed (default: true) |

### Output

```
─── Mock Output (2 lines) ───
  Chat ready. Type 'quit' to exit.
  Hi there! How can I help you today?
─── Pass ✓ ───
```

All `write(stdout, ...)` calls are captured and printed at the end.

## Use Cases

### Unit Testing Agents

Test an agent's logic without LLM costs:

```json
{
  "stdin": ["summarize my code", "quit"],
  "llm_responses": [
    {"content": "", "tool_calls": [{"name": "shell", "arguments": {"command": "cat main.py"}}]},
    "Your code implements a REST API with 3 endpoints."
  ],
  "shell": {
    "cat main.py": "from flask import Flask\napp = Flask(__name__)\n@app.route('/')\ndef home(): return 'hello'"
  }
}
```

### CI/CD

```yaml
# GitHub Actions
- name: Test agents
  run: |
    cognos test examples/chat.cog --env tests/chat-mock.json
    cognos test examples/shell-agent.cog --env tests/shell-mock.json
```

### Regression Testing

Record a trace (`--trace-level full`), then convert it to a mock file to replay:

```bash
# Record
cognos run agent.cog --trace trace.jsonl --trace-level full

# Convert trace to mock (coming soon)
cognos trace-to-mock trace.jsonl > mock.json

# Replay
cognos test agent.cog --env mock.json
```

## Design Principle

The `.cog` file never knows which environment it's running in. It just calls `read()`, `write()`, `think()`, `shell()`. The environment is set by the runner, not the code. Agents are pure logic.
