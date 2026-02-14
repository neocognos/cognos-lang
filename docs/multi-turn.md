# Multi-Turn Conversation Design for Cognos

## Overview

This document describes the multi-turn conversation support added to Cognos. The core concept is that a **Conversation** is a list of messages that accumulates client-side, allowing `think()` to continue existing conversations instead of always starting fresh.

## Core Concept

A conversation is represented as a List of message Maps, where each message has:
- `role` (String): "user", "assistant", or "tool"
- `content` (String or List): The message content or content blocks
- Additional fields for tool use: `tool_calls`, `has_tool_calls`

## API Changes

### think() Function Updates

The `think()` builtin gains new optional keyword arguments:

- **`conversation`** (List of Maps, optional): If provided, the prompt is appended as the next user message and the full conversation array is sent to the model. If `none` (default), behaves like current single-turn behavior.

- **`tool_results`** (List of Maps, optional): Tool result content blocks to append as a user message. Each Map has:
  - `tool_use_id` (String): ID of the tool call being responded to
  - `content` (String): The tool execution result

### think() Return Value Updates

The `think()` function now returns additional fields when used with conversations:

- **`result["conversation"]`** (List): The updated messages array including this turn
- **`result["tool_calls"]`** (List): Tool calls made by the model (if any) with `id`, `name`, `arguments` fields  
- **`result["has_tool_calls"]`** (Bool): Whether the response contains tool calls

## Native Tool Use

When `conversation` is provided AND the model starts with "claude", Cognos uses Anthropic's native `tools` parameter instead of injecting tool descriptions into the system prompt:

- Tools are sent as proper JSON schemas: `{"name": "...", "description": "...", "input_schema": {"type": "object", "properties": {...}}}`
- Tool schemas are automatically built from flow definitions (name, docstring as description, parameters as properties)
- Response parsing handles both `text` and `tool_use` content blocks directly
- `tool_use` blocks contain: `type`, `id`, `name`, `input`

## Message Format

### Single-Turn (Backward Compatible)
```cognos
r = think("Hello", model="claude-sonnet-4-20250514")
# Returns: Value::String("Hi there!")
```

### Multi-Turn with Conversation
```cognos
r = think("Hello", model="claude-sonnet-4-20250514", conversation=[])
# Returns: Value::Map([
#   ("content", "Hi there!"),
#   ("conversation", [
#     {"role": "user", "content": "Hello"},
#     {"role": "assistant", "content": "Hi there!", "has_tool_calls": false}
#   ]),
#   ("has_tool_calls", false)
# ])
```

### With Tool Results
```cognos
tool_results = [{"tool_use_id": "call_123", "content": "File contents here"}]
r = think("", conversation=conv, tool_results=tool_results, tools=["read_file"])
```

## Conversation Management

### Message Structure
Each message in the conversation is a Map with:
- **User messages**: `{"role": "user", "content": "text"}` or `{"role": "user", "content": [content_blocks]}`
- **Assistant messages**: `{"role": "assistant", "content": "text", "has_tool_calls": bool, "tool_calls": [...]}`
- **Tool result messages**: `{"role": "user", "content": [{"type": "tool_result", "tool_use_id": "...", "content": "..."}]}`

### Content Blocks
Content can be either:
- A simple string for text-only messages
- A List of content blocks for multimodal or tool messages:
  - `{"type": "text", "text": "..."}`
  - `{"type": "tool_result", "tool_use_id": "...", "content": "..."}`

## Tool Loop Rewrite

The new `exec.cog` becomes much simpler:

1. Get response from `think()` with conversation
2. If `has_tool_calls == false`, we're done
3. Execute all tool calls and collect results
4. Call `think()` again with updated conversation + tool results
5. Repeat until done or max turns reached

No sliding window, no truncation - the full conversation is preserved and managed by the model provider.

## Max Turn Handling

- **Final turn warning**: On the last allowed turn, append "This is your FINAL turn" to help the model wrap up
- **Return status**: `{"status": "done"|"max_turns"|"error", "turns": N, "conversation": [...]}`
- **Continuation**: Caller can continue by calling `exec()` again with more turns since the full conversation is returned

## Backward Compatibility

- `conversation=none` (default) preserves exact current behavior - single-turn only
- All existing `.cog` files continue to work unchanged
- Only when `conversation=[]` is explicitly provided does multi-turn mode activate

## Implementation Notes

### Anthropic Messages API
- Endpoint: `https://api.anthropic.com/v1/messages`
- Auth: `x-api-key` header with `ANTHROPIC_API_KEY` env var
- Version: `anthropic-version: 2023-06-01`
- Native tool format uses `stop_reason: "tool_use"`

### Content Block Parsing
Mixed content blocks in responses:
```json
[
  {"type": "text", "text": "Let me read that file..."},
  {"type": "tool_use", "id": "call_123", "name": "read_file", "input": {"path": "main.py"}}
]
```

Tool results in user messages:
```json
[
  {"type": "tool_result", "tool_use_id": "call_123", "content": "def main():\n    print('hello')"}
]
```

## Example Usage

### Basic Multi-Turn
```cognos
# Start conversation
r1 = think("What's 2+2?", conversation=[])
conv = r1["conversation"]

# Continue conversation  
r2 = think("What about 3+3?", conversation=conv)
conv = r2["conversation"]
```

### With Tool Use
```cognos
# Start with tool-enabled conversation
r = think("Read main.py and explain it", tools=["read_file"], conversation=[])

if r["has_tool_calls"]:
    # Use exec.cog to handle the tool loop
    result = exec(r["conversation"], tools=["read_file"], max_turns=10)
    final_conv = result["conversation"]
```

This design enables powerful multi-turn interactions while maintaining full backward compatibility with existing Cognos code.