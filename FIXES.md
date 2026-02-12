# FIXES.md — All Bug Fixes & Changes Log

Consolidated record of all bugs found, fixed, and their commits. For review.

---

## BUG-1: Comments as first line of indented block (FIXED)
**Severity**: Medium  
**Commit**: `d1a129b`  
**Symptom**: `flow main():\n    # comment\n    write(stdout, "x")` → parse error  
**Root cause**: Lexer emitted `Indent` + `Newline` for comment-only lines inside blocks, confusing the parser  
**Fix**: In `lexer.rs`, at `at_line_start`, peek ahead to detect blank/comment-only lines. Skip entire line (including trailing `\n`) without emitting any tokens. Stay at `at_line_start`  
**File**: `src/lexer.rs`  
**Test**: `test_comment_first_line_of_block`, `test_comment_nested_block`

## BUG-2: `await` without parentheses (FIXED)
**Severity**: Low  
**Commit**: `d1a129b`  
**Symptom**: `r = await handle` → parse error (only `await(handle)` worked)  
**Fix**: Parser checks for `(` after `await` keyword. Without parens: parse primary expression and wrap in `Expr::Call { name: "await", args: [expr] }`  
**File**: `src/parser.rs`  
**Test**: `test_await_no_parens`

## BUG-3: Integer division `7/2=3` (KNOWN, INTENTIONAL)
**Severity**: Low  
**Status**: Won't fix — matches Python 2 semantics. Could revisit for `//` vs `/` operator distinction  

## BUG-4: `none` displayed as empty string (FIXED)
**Severity**: Medium  
**Commit**: `d1a129b`  
**Symptom**: `f"val={none}"` → `"val="` instead of `"val=none"`  
**Fix**: `Value::None` Display impl changed from `""` to `"none"`  
**File**: `src/interpreter.rs`  
**Test**: `test_none_literal`

## BUG-5: EOF crashes on `read(stdin)` (FIXED)
**Severity**: High  
**Commit**: `7cd196d`  
**Symptom**: `read(stdin)` panics when stdin is closed (EOF)  
**Fix**: `read(stdin)` returns `Value::None` on EOF instead of crashing. Error message changed to "end of input (EOF)" for pattern matching  
**File**: `src/interpreter.rs`  
**Test**: `test_read_stdin_eof_returns_none`

## BUG-6: `none` keyword missing (FIXED)
**Severity**: High  
**Commit**: `7cd196d`  
**Symptom**: No way to compare against None — `if x == none:` was a syntax error  
**Fix**: Added `Token::None_` in lexer, `Expr::NoneLiteral` in AST, evaluates to `Value::None`. `none` is a reserved keyword  
**Files**: `src/token.rs`, `src/lexer.rs`, `src/ast.rs`, `src/parser.rs`, `src/interpreter.rs`  
**Test**: `test_none_literal`

## BUG-7: DeepSeek markdown leakage in tool responses (KNOWN)
**Severity**: Low  
**Status**: Model behavior — DeepSeek sometimes wraps tool call JSON in markdown fences. Mitigated by `extract_json()` bracket-matching fallback  

## BUG-8: `break` in `select:` doesn't propagate to outer `loop:` (FIXED)
**Severity**: Critical  
**Commit**: `35e3cce`  
**Symptom**: Event-driven pattern `loop: select: branch: ... break` loops forever — break only exits select, not loop  
**Root cause**: `run_select()` returned `Result<()>`, discarding ControlFlow from branches  
**Fix**: `run_select()` returns `Result<ControlFlow>`. Branch threads track ControlFlow (Break/Return) and send through mpsc channel as third tuple element. Winning branch's ControlFlow propagates to caller  
**File**: `src/interpreter.rs`  
**Test**: `test_select_break_propagates`

## BUG-9: No conversation history in general-assistant (FIXED)
**Severity**: Medium  
**Commit**: `35e3cce`  
**Symptom**: Each LLM call had no context of previous turns  
**Fix**: `respond()` flow accepts `history: List` param, joins into context string. Main loop accumulates `history = history + [f"User: {input}"]`  
**File**: `examples/general-assistant.cog`

## BUG-10: All examples use expensive Claude model (FIXED)
**Severity**: Medium  
**Commit**: `35e3cce`  
**Symptom**: Examples hardcoded `claude-sonnet-4-20250514` — expensive and slow via CLI  
**Fix**: Switched all examples to `deepseek-chat` — native tool_use, ~2-3s, affordable  
**Files**: `examples/devops-agent.cog`, `examples/research-agent.cog`, `examples/review-agent.cog`, `examples/shell-agent.cog`, `examples/data-pipeline.cog`

## BUG-11: F-strings can't have quotes inside `{}` (KNOWN)
**Severity**: Low  
**Status**: Documented limitation. `f"{m['key']}"` fails — use `f"{m.key}"` or extract to variable  

## BUG-12: Default model too weak (FIXED)
**Severity**: Medium  
**Commit**: `35e3cce`  
**Symptom**: Default `qwen2.5:1.5b` too weak for tool calling  
**Fix**: Changed to `qwen2.5:7b`. Configurable via `COGNOS_MODEL` env var  
**File**: `src/interpreter.rs`

## BUG-13: All examples hardcoded to Claude (FIXED)
**Severity**: Medium  
**Commit**: `35e3cce`  
**Same as BUG-10, tracked separately in ISSUES-2.md

---

## Test Harness Fixes (not Cognos bugs)

### Harness: `grep -F` for literal matching
**Commit**: `38b09c6`  
**Symptom**: `grep` treated `-5` as a flag  
**Fix**: `grep -qF --` for all expect matching

### Harness: Shell quoting in inline tests
**Commit**: `38b09c6`  
**Symptom**: Single quotes in heredoc strings, double quotes in prompts broke shell expansion  
**Fix**: Removed problematic quote characters from test prompts, used simpler string forms

### Harness: Import paths relative to temp files
**Commit**: `38b09c6`  
**Symptom**: `import "examples/lib/..."` failed when test file was in `/tmp/`  
**Fix**: Replaced inline import tests with `run_test` using real example files

---

## Feature Commits (non-bug)

| Commit | Feature |
|--------|---------|
| `3aad034` | DeepSeek provider — `deepseek-*` via OpenAI-compatible API |
| `b459c99` | General assistant switched to DeepSeek |
| `7cd196d` | `none` keyword + EOF handling |
| `d1a129b` | Batch: comments, await syntax, none display, examples updated |
| `35e3cce` | Batch: select break, history, default model, DeepSeek examples |
| `cebffd5` | Test harness v1 (65 static tests) |
| `38b09c6` | Test harness v2 (71 tests, randomized per run) |

---

## Test Coverage Summary

- **215 unit/integration tests** (Rust `cargo test`)
- **71 harness tests** (shell-based, randomized per run):
  - 8 basic programs
  - 46 language features
  - 6 error cases  
  - 5 randomized LLM tests (DeepSeek)
  - 2 randomized agent sessions
  - 10 fuzz/stress tests
  - 2 chat sessions with varied inputs

All green as of `38b09c6`.
