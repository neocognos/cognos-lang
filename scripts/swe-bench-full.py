#!/usr/bin/env python3
"""Run meta-agent-v3 on SWE-bench Lite (300 instances).
Usage: python3 scripts/swe-bench-full.py [--start N] [--limit N] [--instance ID]
"""
import json, subprocess, os, sys, time, argparse
from pathlib import Path

COGNOS = os.path.expanduser("~/clawd/neocognos/cognos-lang/target/release/cognos")
COGNOS_DIR = os.path.expanduser("~/clawd/neocognos/cognos-lang")
AGENT = os.path.join(COGNOS_DIR, "examples/meta-agent-v3.cog")
RESULTS_DIR = os.path.expanduser("~/clawd/neocognos/cognos-lang/swe-results")
REPOS_DIR = "/tmp/swe-repos"

def load_dataset():
    """Load SWE-bench Lite dataset."""
    from datasets import load_dataset
    ds = load_dataset('princeton-nlp/SWE-bench_Lite', split='test')
    return list(ds)

def setup_repo(instance):
    """Clone and checkout the correct commit for an instance."""
    repo = instance['repo']
    commit = instance['base_commit']
    iid = instance['instance_id']
    repo_path = os.path.join(REPOS_DIR, iid.replace('/', '__'))
    
    if not os.path.exists(repo_path):
        print(f"  Cloning {repo}...", flush=True)
        r = subprocess.run(
            ["git", "clone", "--depth", "200", f"https://github.com/{repo}.git", repo_path],
            capture_output=True, text=True, timeout=180
        )
        if r.returncode != 0:
            # Try full clone for repos that need deeper history
            subprocess.run(
                ["git", "clone", f"https://github.com/{repo}.git", repo_path],
                capture_output=True, text=True, timeout=300
            )
    
    # Checkout the base commit
    subprocess.run(["git", "checkout", commit], cwd=repo_path, capture_output=True, text=True)
    subprocess.run(["git", "checkout", "--", "."], cwd=repo_path, capture_output=True, text=True)
    subprocess.run(["git", "clean", "-fd"], cwd=repo_path, capture_output=True, text=True)
    
    return repo_path

def run_instance(instance, timeout=300):
    """Run the meta-agent on a single instance."""
    iid = instance['instance_id']
    problem = instance['problem_statement']
    
    # Setup repo
    try:
        repo_path = setup_repo(instance)
    except Exception as e:
        return {"id": iid, "status": "CLONE_FAILED", "error": str(e)}
    
    # Write issue and repo path
    with open("/tmp/cognos-issue.txt", "w") as f:
        f.write(problem)
    with open("/tmp/cognos-repo.txt", "w") as f:
        f.write(repo_path)
    
    # Run meta-agent
    env = os.environ.copy()
    env.pop('ANTHROPIC_API_KEY', None)
    
    start_time = time.time()
    try:
        r = subprocess.run(
            [COGNOS, "run", "--allow-shell", AGENT],
            capture_output=True, text=True, timeout=timeout,
            cwd=COGNOS_DIR, env=env
        )
        output = r.stdout + r.stderr
        exit_code = r.returncode
    except subprocess.TimeoutExpired:
        output = "TIMEOUT"
        exit_code = -1
    
    elapsed = time.time() - start_time
    
    # Check for patch
    diff = subprocess.run(["git", "diff"], cwd=repo_path, capture_output=True, text=True)
    patch = diff.stdout.strip()
    
    # Extract mode from output
    mode = "unknown"
    for line in output.split("\n"):
        if "Phase 1:" in line:
            mode = "v3-scout"
        elif "Falling back to agent" in line:
            mode = "v3-fallback"
    
    result = {
        "id": iid,
        "status": "PATCH" if (patch and len(patch) > 10) else "NO_PATCH",
        "patch_size": len(patch) if patch else 0,
        "mode": mode,
        "time": round(elapsed, 1),
        "exit_code": exit_code,
    }
    
    # Save patch
    if patch and len(patch) > 10:
        patch_file = os.path.join(RESULTS_DIR, "patches", f"{iid}.patch")
        os.makedirs(os.path.dirname(patch_file), exist_ok=True)
        with open(patch_file, "w") as f:
            f.write(patch)
    
    # Save output
    log_file = os.path.join(RESULTS_DIR, "logs", f"{iid}.log")
    os.makedirs(os.path.dirname(log_file), exist_ok=True)
    with open(log_file, "w") as f:
        f.write(output)
    
    return result

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--limit", type=int, default=300)
    parser.add_argument("--instance", type=str, default=None)
    parser.add_argument("--timeout", type=int, default=300)
    args = parser.parse_args()
    
    os.makedirs(RESULTS_DIR, exist_ok=True)
    os.makedirs(REPOS_DIR, exist_ok=True)
    
    print("Loading SWE-bench Lite...", flush=True)
    dataset = load_dataset()
    print(f"Loaded {len(dataset)} instances", flush=True)
    
    if args.instance:
        dataset = [d for d in dataset if d['instance_id'] == args.instance]
        if not dataset:
            print(f"Instance {args.instance} not found")
            return
    else:
        dataset = dataset[args.start:args.start + args.limit]
    
    results = []
    results_file = os.path.join(RESULTS_DIR, "results.jsonl")
    
    for i, instance in enumerate(dataset):
        iid = instance['instance_id']
        print(f"\n{'='*60}", flush=True)
        print(f"[{i+1}/{len(dataset)}] {iid}", flush=True)
        
        # Check if already done
        if os.path.exists(os.path.join(RESULTS_DIR, "logs", f"{iid}.log")):
            print("  Already done, skipping.", flush=True)
            continue
        
        result = run_instance(instance, timeout=args.timeout)
        results.append(result)
        
        emoji = "✅" if result["status"] == "PATCH" else "❌"
        print(f"  {emoji} {result['status']} ({result['time']}s, {result['mode']})", flush=True)
        
        # Append to results file
        with open(results_file, "a") as f:
            f.write(json.dumps(result) + "\n")
        
        # Brief pause to avoid rate limits
        time.sleep(2)
    
    # Summary
    print(f"\n{'='*60}", flush=True)
    print("SUMMARY:", flush=True)
    patches = sum(1 for r in results if r["status"] == "PATCH")
    total = len(results)
    print(f"  Patches: {patches}/{total} ({100*patches/total:.0f}%)" if total else "  No results", flush=True)

if __name__ == "__main__":
    main()
