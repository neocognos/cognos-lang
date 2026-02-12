# Issues Found — Extensive Testing Session (2026-02-12)

## BUG-1: Comments as first line of indented block fail
**Severity**: Medium
**Repro**: 
```cognos
flow main():
    # this comment crashes
    write(stdout, "hello")
```
**Error**: `Parse error: line 2: expected indent, got newline`
**Cause**: Lexer emits Newline for comment-only lines before emitting Indent. Parser sees Newline where it expects Indent.
**Fix**: Lexer should skip comment-only lines when determining indentation, or treat them as blank lines.

## BUG-2: `await` requires parentheses — `await handle` doesn't parse
**Severity**: Medium  
**Repro**:
```cognos
handle = async slow(3)
result = await handle   # FAILS: expected '(', got 'handle'
result = await(handle)  # WORKS
```
**Cause**: Parser treats `await` like a function call via `parse_call()`, which expects `(`.
**Fix**: In `parse_primary`, when `Token::Await` is seen, parse the next expression instead of calling `parse_call`.

## BUG-3: Integer division truncates — `7 / 2 = 3`
**Severity**: Low (debatable)
**Current**: `/` does integer division when both operands are Int.
**Expected**: `/` should return Float (like Python). Use explicit `//` for integer division if needed.
**Decision needed**: Is this intentional? Python does `7/2=3.5`, `7//2=3`. Cognos currently acts like Python 2.

## BUG-4: `none` displays as empty string in f-strings
**Severity**: Low
**Repro**: `write(stdout, f"value: {none}")` → `value: ` (empty)
**Expected**: `value: none` — Python shows `None`
**Fix**: Update Value::None Display impl to print "none".

## BUG-5: `chat.cog` and other examples missing EOF handling
**Severity**: Low
**Files**: `chat.cog`, `tool-agent.cog`, `shell-agent.cog`
**Fix**: Add `if input == none:` checks after `read()` calls.

## BUG-6: `exec()` only handles first tool call
**Severity**: Medium
**Repro**: When LLM returns multiple tool_calls, `exec()` in `lib/exec.cog` may only process the first.
**Needs verification**: Check if `exec()` iterates all tool_calls or just the first.

## BUG-7: DeepSeek sometimes leaks markdown in tool-use responses
**Severity**: Low
**Observed**: When asked "list files in /tmp ending with .cog", the final answer included a markdown code block showing the command instead of just the results.
**Cause**: This is a model behavior issue, not a Cognos bug. Could be mitigated with system prompt tuning.

---

## Summary
| Bug | Severity | Effort | Priority |
|-----|----------|--------|----------|
| BUG-1 | Medium | Medium | High — breaks real programs |
| BUG-2 | Medium | Low | High — async/await is core feature |
| BUG-3 | Low | Low | Medium — decide semantics |
| BUG-4 | Low | Trivial | Medium — confusing UX |
| BUG-5 | Low | Trivial | Low — example cleanup |
| BUG-6 | Medium | Low | Medium — verify first |
| BUG-7 | Low | N/A | Low — model behavior |
