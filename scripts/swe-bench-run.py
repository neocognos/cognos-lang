#!/usr/bin/env python3
"""Run Cognos coding agent on SWE-bench instances."""
import json
import subprocess
import sys
import os
import tempfile
from datasets import load_dataset

COGNOS = os.path.expanduser("~/clawd/neocognos/cognos-lang/target/release/cognos")
COGNOS_DIR = os.path.expanduser("~/clawd/neocognos/cognos-lang")

AGENTS = {
    "coding": os.path.join(COGNOS_DIR, "examples/coding-agent-opus.cog"),
    "meta": os.path.join(COGNOS_DIR, "examples/meta-agent.cog"),
    "meta-multi": os.path.join(COGNOS_DIR, "examples/meta-agent-multi.cog"),
}
AGENT = AGENTS["coding"]  # default

def run_instance(instance, timeout=600):
    """Run the coding agent on a single SWE-bench instance."""
    instance_id = instance["instance_id"]
    repo = instance["repo"]
    base_commit = instance["base_commit"]
    problem = instance["problem_statement"]
    
    print(f"\n{'='*60}")
    print(f"Instance: {instance_id}")
    print(f"Repo: {repo}, Commit: {base_commit[:12]}")
    print(f"Problem: {problem[:100]}...")
    print(f"{'='*60}")
    
    # Clone repo at the right commit
    with tempfile.TemporaryDirectory(prefix="swe-bench-") as tmpdir:
        repo_path = os.path.join(tmpdir, "repo")
        
        # Clone
        print(f"Cloning {repo}...")
        result = subprocess.run(
            ["git", "clone", "--depth", "100", f"https://github.com/{repo}.git", repo_path],
            capture_output=True, text=True, timeout=120
        )
        if result.returncode != 0:
            print(f"Clone failed: {result.stderr[:200]}")
            return {"instance_id": instance_id, "model_name_or_path": "cognos-agent", "model_patch": ""}
        
        # Checkout base commit
        subprocess.run(
            ["git", "checkout", base_commit],
            cwd=repo_path, capture_output=True, text=True
        )
        
        # Run the Cognos agent
        print("Running Cognos agent...")
        env = os.environ.copy()
        # Source .env file
        env_file = os.path.join(COGNOS_DIR, ".env")
        if os.path.exists(env_file):
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        key, val = line.split("=", 1)
                        env[key] = val
        
        # Write issue and repo path to files for the agent
        with open("/tmp/cognos-issue.txt", "w") as f:
            f.write(problem)
        with open("/tmp/cognos-repo.txt", "w") as f:
            f.write(repo_path)
        
        try:
            result = subprocess.run(
                [COGNOS, "run", "--memory", "--allow-shell", "-vv",
                 "--trace", f"/tmp/cognos-swe-trace-{instance_id}.jsonl",
                 "--trace-level", "full",
                 "--memory-db", f"/tmp/cognos-swe-{instance_id}.db",
                 "--memory-ns", instance_id,
                 AGENT],
                input="",
                capture_output=True, text=True,
                timeout=timeout,
                cwd=COGNOS_DIR,
                env=env
            )
            output = result.stdout
            stderr = result.stderr
        except subprocess.TimeoutExpired:
            print(f"TIMEOUT after {timeout}s")
            output = ""
            stderr = "timeout"
        
        # Extract diff from output
        patch = ""
        if "--- PATCH ---" in output:
            start = output.index("--- PATCH ---") + len("--- PATCH ---")
            end = output.index("--- END PATCH ---") if "--- END PATCH ---" in output else len(output)
            patch = output[start:end].strip()
        
        # Also try getting diff directly from repo
        if not patch:
            diff_result = subprocess.run(
                ["git", "diff"], cwd=repo_path, capture_output=True, text=True
            )
            patch = diff_result.stdout.strip()
        
        # Clean up memory db
        try:
            os.remove(f"/tmp/cognos-swe-{instance_id}.db")
        except:
            pass
        
        if stderr:
            print(f"Agent stderr: {stderr[:500]}")
        if output:
            print(f"Agent stdout: {output[:500]}")
        print(f"Patch length: {len(patch)} chars")
        if patch:
            print(f"Patch preview: {patch[:200]}...")
        else:
            print("No patch generated")
        
        return {
            "instance_id": instance_id,
            "model_name_or_path": "cognos-agent",
            "model_patch": patch
        }

def estimate_difficulty(instance):
    """Rough heuristic: shorter problem statements with clear error messages are easier."""
    problem = instance["problem_statement"]
    score = 0
    # Short problems tend to be simpler
    if len(problem) < 500:
        score += 2
    elif len(problem) < 1000:
        score += 1
    # Clear error traces help
    if "TypeError" in problem or "AttributeError" in problem or "ValueError" in problem:
        score += 2
    if "Traceback" in problem:
        score += 1
    # Single-file repos are easier
    repo = instance["repo"]
    if repo.startswith("psf/requests") or repo.startswith("pallets/flask"):
        score += 1
    return score

def main():
    # Load dataset
    ds = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")
    
    # Parse args
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    output_file = sys.argv[3] if len(sys.argv) > 3 else "/tmp/cognos-swe-predictions.jsonl"
    easy_first = "--easy" in sys.argv
    
    # Agent selection
    agent_name = "coding"
    for arg in sys.argv:
        if arg.startswith("--agent="):
            agent_name = arg.split("=", 1)[1]
    if agent_name in AGENTS:
        global AGENT
        AGENT = AGENTS[agent_name]
        print(f"Using agent: {agent_name} ({AGENT})")
    else:
        print(f"Unknown agent '{agent_name}', available: {list(AGENTS.keys())}")
        sys.exit(1)
    
    if easy_first:
        # Sort by estimated difficulty (easiest first)
        indices = sorted(range(len(ds)), key=lambda i: -estimate_difficulty(ds[i]))
        print(f"Running {n} EASIEST instances (sorted by heuristic)")
    else:
        indices = list(range(len(ds)))
        print(f"Running {n} instances starting from {start}")
    
    print(f"Output: {output_file}")
    
    predictions = []
    if easy_first:
        run_indices = indices[start:start + n]
    else:
        run_indices = list(range(start, min(start + n, len(ds))))
    
    for i in run_indices:
        pred = run_instance(ds[i])
        predictions.append(pred)
        
        # Append to file incrementally
        with open(output_file, "a") as f:
            f.write(json.dumps(pred) + "\n")
        
        print(f"\nProgress: {len(predictions)}/{n}")
    
    # Summary
    patches = sum(1 for p in predictions if p["model_patch"])
    print(f"\n{'='*60}")
    print(f"SUMMARY: {patches}/{len(predictions)} instances produced patches")
    print(f"Predictions saved to: {output_file}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
