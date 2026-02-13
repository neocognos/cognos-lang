# SWE-bench Evaluation Setup

## What It Is
SWE-bench evaluates coding agents by giving them real GitHub issues, applying their generated patches, and running the repo's test suite to verify the fix.

## Requirements
- Docker (for containerized evaluation)
- Python 3.x + `pip install swebench`
- Predictions in JSONL format:
  ```json
  {"instance_id": "sympy__sympy-20590", "model_name_or_path": "cognos-agent", "model_patch": "diff --git a/..."}
  ```

## Datasets
- **SWE-bench Lite**: 300 instances (recommended starting point)
- **SWE-bench Verified**: Human-verified subset
- **SWE-bench Full**: 2,294 instances

## How to Run
```bash
pip install swebench

# Evaluate predictions
python -m swebench.harness.run_evaluation \
  --dataset_name princeton-nlp/SWE-bench_Lite \
  --predictions_path predictions.jsonl \
  --max_workers 8 \
  --run_id cognos-v1
```

## What Our Agent Needs to Produce
For each instance:
1. Read the issue description
2. Explore the repo (find relevant files)
3. Generate a patch (unified diff format)
4. Output JSONL with `instance_id`, `model_name_or_path`, `model_patch`

## Key Metrics
- **Resolution rate**: % of issues where the patch passes all tests
- Current SOTA: ~50-60% on Lite (top agents with Claude/GPT-4)

## Integration with Cognos
A Cognos coding agent would:
1. `think()` to understand the issue
2. `exec(shell)` to explore the repo
3. `think()` with file contents to plan the fix
4. `exec(shell)` to apply and test the patch
5. Output the diff

The agent flow would be a `.cog` file; the SWE-bench harness calls it per-instance.
