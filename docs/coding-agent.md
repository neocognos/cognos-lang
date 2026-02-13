# Cognos Coding Agent

## What We're Building

A competitive autonomous coding agent written entirely in Cognos (`.cog`). The agent takes a GitHub issue description + repository, and produces a working patch that resolves the issue.

**Primary goal**: stress-test and improve the Cognos framework. Every gap the agent exposes becomes a framework improvement.

**Secondary goal**: demonstrate Cognos can express a SOTA coding agent cleanly, and benchmark it against SWE-bench.

## Why Cognos

Existing coding agents (Devin, OpenHands, SWE-Agent) are built with Python + LangChain/LangGraph + ad-hoc tool wrappers. They work, but:

- **Opaque**: thousands of lines of Python scaffolding hiding simple patterns
- **Untestable**: no way to mock LLM calls and verify agent logic
- **Fragile**: one framework update breaks everything
- **Heavyweight**: dozens of dependencies

Cognos offers:

- **Readable**: the entire agent is one `.cog` file you can read top-to-bottom
- **Testable**: `cognos test agent.cog --env mock.json` — deterministic, no LLM needed
- **Traceable**: every LLM call, tool use, decision logged in JSONL
- **Sandboxed**: shell access controlled by policy in the `.cog` file
- **Multi-model**: cheap model for exploration, expensive model for reasoning — per-call

If Cognos can express a competitive coding agent cleanly, the framework is solid.

## Architecture

### Core Loop: Plan → Act → Observe → Remember

```
flow solve(issue: String, repo_path: String):
    # Understand the issue
    remember(f"Issue: {issue}")
    
    # Explore the repo (find relevant files)
    explore(repo_path, issue)
    
    # Plan the fix
    plan = plan_fix(issue)
    
    # Implement the fix
    apply_fix(plan)
    
    # Verify: run tests
    loop:
        result = run_tests(repo_path)
        if result.passed:
            break
        # Self-correct: analyze failure, try different approach
        fix_attempt(result.error, plan)
    
    # Generate diff
    return generate_diff(repo_path)
```

### Design Choices

**1. Memory over compaction**

Other agents keep a growing conversation history and "compact" it when it gets too long. We don't. Instead:

- The agent `remember()`s key findings: "auth logic is in src/lib/auth.ts", "test X fails because of null check"
- Each reasoning step starts fresh with `recall()`ed context relevant to the current sub-problem
- No history bloat, no information loss from summarization
- Memory persists across retries — if the agent crashes and restarts, it still knows what it found

This is structurally better than compaction because:
- Compaction is lossy (the summarizer might drop critical details)
- Memory is selective (only important findings are stored)
- Memory is searchable (recall by relevance, not position in history)

**2. Two-phase reasoning (knowledge vs action)**

From the Slack agent work: don't offer tools when the agent can answer from memory. Phase 1 tries without tools; Phase 2 adds tools only if needed. Prevents the model from reaching for grep when it already knows the answer.

**3. Multi-model routing**

- **Exploration/planning**: DeepSeek Chat (~$0.02/task, fast)
- **Code generation/reasoning**: Claude Sonnet via API (higher quality for actual edits)
- **Repo map generation**: local Ollama (free, no latency concerns)

The agent picks the right model per-call. A single `think()` call specifies `model=`.

**4. Repo map via shell**

Instead of building tree-sitter into the Cognos runtime (P11 violation), the agent calls shell tools:
- `find` + `grep` for file discovery
- `head`/`cat` for reading relevant sections
- `git diff` for generating patches

The "repo map" is built by the agent's exploration flow, not a special builtin. This keeps the runtime lean and the intelligence in `.cog` code.

**5. Reflexion (attempt tracking)**

The agent remembers what it tried:
```
remember(f"Attempt 1: {approach} — failed because {error}")
```

Before each retry, it recalls past attempts:
```
past = recall("attempt failed", limit=5)
```

This prevents the "repair loop of death" where the agent tries the same fix repeatedly.

**6. Self-verification via TDD**

Before declaring success, the agent:
1. Runs the repo's existing test suite
2. Checks that previously-passing tests still pass (no regressions)
3. Checks that the specific failing test now passes

This is the single most impactful reliability technique (>89% F1 with TDD vs <60% without, per research).

## SWE-bench Integration

The agent produces JSONL output:
```json
{"instance_id": "sympy__sympy-20590", "model_name_or_path": "cognos-agent", "model_patch": "diff --git a/..."}
```

A wrapper script feeds SWE-bench instances to the agent and collects patches:
```bash
for instance in swe-bench-lite/*.json; do
    cognos run coding-agent.cog --input "$instance" >> predictions.jsonl
done

python -m swebench.harness.run_evaluation \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --predictions_path predictions.jsonl
```

## Target Metrics

- **SWE-bench Lite**: >30% resolution rate (v1 target)
- **Cost per task**: <$0.10 (using DeepSeek for exploration, Claude for edits)
- **Agent code size**: <200 lines of `.cog`

## What This Exercises in the Framework

| Framework Feature | How the Agent Uses It |
|---|---|
| `think()` with tools | Planning, code generation |
| `exec()` with shell | File exploration, test running, git |
| `remember()`/`recall()` | Codebase knowledge, attempt tracking |
| `try/catch` | Error recovery from failed edits |
| Multi-model routing | Cheap exploration, expensive reasoning |
| `import` | Shared library flows (exec, etc.) |
| Two-phase response | Knowledge vs action separation |
| Tracing | Debug and benchmark every LLM call |
| `cognos test --env mock` | Deterministic testing of agent logic |

## Files

- `examples/coding-agent.cog` — the agent
- `examples/mocks/coding-agent-test.json` — mock for deterministic testing
- `docs/coding-agent.md` — this document
- `docs/swe-bench-setup.md` — benchmark setup
- `docs/research-coding-agents-2026.md` — Gemini research report
