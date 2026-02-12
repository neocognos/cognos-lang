# Issues Found — Extensive Session Testing (2026-02-12, Round 2)

## BUG-8: EOF in `select:` doesn't break outer loop (CRITICAL)
**Severity**: High
**Repro**: `general-assistant.cog` — after "exit" input, the `select:` block's stdin branch gets EOF, `none` check triggers `break`, but `break` only exits the `select:`, not the outer `loop:`.
**Effect**: Infinite `> > > > ...` loop after exit when background tasks are active.
**Fix**: Need `break` to propagate through `select:` to outer loop. Or use a flag variable.
**Workaround**: Set a `done` variable and check it after `select:`.

## BUG-9: No conversational context in general-assistant
**Severity**: Medium
**Observed**: "Read back the file you just created" → "I don't see any record"
**Cause**: `general-assistant.cog` doesn't maintain conversation history. Each `think()` call is stateless.
**Fix**: Add history accumulation like `chat.cog` does.

## BUG-11: F-strings don't support string literals inside `{}`
**Severity**: Medium
**Repro**: `f"value: {m['key']}"` → error: undefined variable 'key'
**Cause**: F-string expression parser doesn't handle quotes inside braces (the lexer can't distinguish nested quotes).
**Workaround**: Extract to variable first: `v = m["key"]` then `f"value: {v}"`. Or use dot access: `f"value: {m.key}"`.
**Fix**: Document limitation OR implement nested quote parsing in f-string lexer.

## BUG-12: Default model `qwen2.5:1.5b` is too small
**Severity**: Medium
**Current**: `think()` without `model=` uses `qwen2.5:1.5b` which fails at tool use.
**Fix**: Change default to `qwen2.5:7b` (best local model) or make configurable via env var `COGNOS_MODEL`.

## BUG-13: Many examples hardcode `claude-sonnet-4-20250514`
**Severity**: Low
**Files**: `devops-agent.cog`, `research-agent.cog`, `review-agent.cog`, `shell-agent.cog`, `data-pipeline.cog`
**Effect**: Examples fail when Claude CLI is not logged in.
**Fix**: Switch to `deepseek-chat` or add `COGNOS_MODEL` env var support.

---

## Summary
| Bug | Severity | Effort | Priority |
|-----|----------|--------|----------|
| BUG-8  | High | Medium | **P0** — breaks select+loop exit |
| BUG-9  | Medium | Low | P1 — usability |
| BUG-11 | Medium | Hard | P2 — document as limitation |
| BUG-12 | Medium | Trivial | **P0** — wrong default model |
| BUG-13 | Low | Trivial | P1 — example maintenance |
