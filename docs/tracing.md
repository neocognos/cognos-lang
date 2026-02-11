# Tracing & Diagnostics

Cognos has built-in structured tracing for monitoring and troubleshooting agent execution.

## Quick Start

```bash
cognos run --trace trace.jsonl file.cog
```

Every runtime event is written as one JSON line to the trace file.

## Trace Events

### llm_call

Emitted for every `think()` call.

```json
{
  "ts": "1770852240",
  "elapsed_ms": 4066,
  "turn": 1,
  "event": "llm_call",
  "model": "claude-sonnet-4-20250514",
  "provider": "claude-cli",
  "latency_ms": 4066,
  "prompt_chars": 27,
  "response_chars": 115,
  "has_tool_calls": true,
  "error": null
}
```

| Field | Description |
|-------|-------------|
| `model` | Model name passed to `think()` |
| `provider` | `claude-cli`, `anthropic`, or `ollama` |
| `latency_ms` | Time from request to response |
| `prompt_chars` | Characters sent to the LLM |
| `response_chars` | Characters received |
| `has_tool_calls` | Whether the LLM requested tool calls |
| `error` | Error message if the call failed, null otherwise |

### shell_exec

Emitted for every `__exec_shell__()` call.

```json
{
  "event": "shell_exec",
  "command": "date | head -50",
  "latency_ms": 2,
  "exit_code": 0,
  "output_chars": 32
}
```

| Field | Description |
|-------|-------------|
| `command` | The shell command executed |
| `latency_ms` | Execution time |
| `exit_code` | Process exit code (0 = success) |
| `output_chars` | Characters in stdout |

### tool_exec

Emitted when `exec()` invokes a tool flow.

```json
{
  "event": "tool_exec",
  "tool": "search",
  "args": "{\"query\": \"weather amsterdam\"}",
  "latency_ms": 150,
  "result_chars": 200,
  "success": true,
  "error": null
}
```

### flow_start / flow_end

Emitted when a flow begins and ends execution.

```json
{"event": "flow_start", "flow": "main"}
{"event": "flow_end", "flow": "main", "duration_ms": 15000}
```

### io

Emitted for `read()` and `write()` operations.

```json
{
  "event": "io",
  "op": "read",
  "handle": "file",
  "path": "data.txt",
  "bytes": 4096
}
```

### context

Emitted to track conversation context growth.

```json
{
  "event": "context",
  "history_len": 8,
  "context_chars": 3200
}
```

### error

Emitted when a runtime error occurs.

```json
{
  "event": "error",
  "category": "runtime",
  "message": "cannot iterate over 42 (type: Int)",
  "flow": "main"
}
```

## Common Fields

Every event includes:

| Field | Description |
|-------|-------------|
| `ts` | Unix timestamp (seconds) |
| `elapsed_ms` | Milliseconds since program start |
| `turn` | Conversation turn number |

## Analyzing Traces

### With jq

```bash
# All LLM calls with latency
jq 'select(.event == "llm_call") | {model, latency_ms, prompt_chars, response_chars}' trace.jsonl

# Total LLM time
jq -s '[.[] | select(.event == "llm_call") | .latency_ms] | add' trace.jsonl

# Tool calls only
jq 'select(.event == "tool_exec")' trace.jsonl

# Errors
jq 'select(.event == "error")' trace.jsonl

# Shell commands with exit codes
jq 'select(.event == "shell_exec") | {command, exit_code, latency_ms}' trace.jsonl
```

### Summary report

```bash
# Count events by type
jq -s 'group_by(.event) | map({event: .[0].event, count: length})' trace.jsonl

# Average LLM latency by provider
jq -s '[.[] | select(.event == "llm_call")] | group_by(.provider) | map({provider: .[0].provider, avg_ms: (map(.latency_ms) | add / length)})' trace.jsonl
```

## CLI Reference

```
cognos run --trace <path> [--allow-shell] <file.cog>
```

| Flag | Description |
|------|-------------|
| `--trace <path>` | Write JSONL trace events to file |
| `--allow-shell` | Enable `__exec_shell__()` primitive |
| `-v` / `-vv` / `-vvv` | Log verbosity (info/debug/trace) to stderr |

Tracing and logging are independent — you can use both:

```bash
cognos run -v --trace trace.jsonl --allow-shell agent.cog
```
