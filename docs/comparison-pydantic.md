# Cognos vs PydanticAI — Side-by-Side Comparison

The same agent: analyze source files, validate structured output, generate project summary.

---

## Cognos (68 lines)

```cognos
import "lib/validated_think.cog"

type FileAnalysis:
    filename: String
    purpose: String
    complexity: Int
    issues: List
    suggestions: List

type ProjectSummary:
    total_files: Int
    total_lines: Int
    architecture: String
    top_issues: List
    overall_score: Int

flow shell(command: String) -> String:
    "Execute a sandboxed shell command."
    return __exec_shell__(f"{command} | head -50")

flow read_file(path: String) -> String:
    "Read the contents of a file"
    return read(file(path))

flow main():
    files_raw = shell("find src -name '*.rs' -type f | sort | head -3")
    files = files_raw.split("\n")

    analyses = []
    for filepath in files:
        if filepath == "":
            continue
        try:
            content = read_file(filepath)
            preview = content[0:2000]
            analysis = validated_think(
                f"File: {filepath}\n\n{preview}",
                "FileAnalysis",
                "claude-sonnet-4-20250514",
                "Analyze this source file. Rate complexity 1-10.",
                "qwen2.5:1.5b"
            )
            analyses = analyses + [analysis]
        catch err:
            write(stdout, f"Error: {err}")

    summary = validated_think(
        f"Project with {files.length} files:\n{analyses}",
        "ProjectSummary",
        "claude-sonnet-4-20250514",
        "Summarize this project analysis. Score 1-10.",
        "qwen2.5:1.5b"
    )

    save("analysis-report.json", summary)
```

**Test:**
```bash
cognos test code-analyzer.cog --env mocks/code-analyzer-sonnet.json
```

---

## PydanticAI (145 lines)

```python
import asyncio
import json
import subprocess
from pathlib import Path
from typing import List

from pydantic import BaseModel, Field
from pydantic_ai import Agent, RunContext

# --- Types (same as Cognos type definitions) ---

class FileAnalysis(BaseModel):
    filename: str
    purpose: str
    complexity: int = Field(ge=1, le=10)
    issues: List[str]
    suggestions: List[str]

class ProjectSummary(BaseModel):
    total_files: int
    total_lines: int
    architecture: str
    top_issues: List[str]
    overall_score: int = Field(ge=1, le=10)

# --- Agent setup ---

file_analyzer = Agent(
    "anthropic:claude-sonnet-4-20250514",
    result_type=FileAnalysis,
    system_prompt="Analyze this source file. Rate complexity 1-10.",
    retries=3,
)

project_summarizer = Agent(
    "anthropic:claude-sonnet-4-20250514",
    result_type=ProjectSummary,
    system_prompt="Summarize this project analysis. Score 1-10.",
    retries=3,
)

# --- Tools (registered as decorators) ---

@file_analyzer.tool
async def shell(ctx: RunContext, command: str) -> str:
    """Execute a sandboxed shell command."""
    result = subprocess.run(
        f"{command} | head -50",
        shell=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    return result.stdout

@file_analyzer.tool
async def read_file(ctx: RunContext, path: str) -> str:
    """Read the contents of a file."""
    return Path(path).read_text()

# --- Main flow ---

async def main():
    # Step 1: Discover files
    result = subprocess.run(
        "find src -name '*.rs' -type f | sort | head -3",
        shell=True,
        capture_output=True,
        text=True,
    )
    files = [f for f in result.stdout.strip().split("\n") if f]

    # Step 2: Analyze each file
    analyses = []
    for filepath in files:
        try:
            content = Path(filepath).read_text()
            preview = content[:2000]
            result = await file_analyzer.run(
                f"File: {filepath}\n\n{preview}"
            )
            analyses.append(result.data)
        except Exception as e:
            print(f"Error: {e}")

    # Step 3: Generate project summary
    analyses_text = "\n".join(
        f"- {a.filename}: complexity={a.complexity}, issues={a.issues}"
        for a in analyses
    )
    result = await project_summarizer.run(
        f"Project with {len(files)} files:\n{analyses_text}"
    )
    summary = result.data

    # Save results
    Path("analysis-report.json").write_text(
        json.dumps(summary.model_dump(), indent=2)
    )

if __name__ == "__main__":
    asyncio.run(main())
```

**Test:**
```python
# Requires: pytest, pytest-asyncio, custom mock setup
import pytest
from unittest.mock import AsyncMock, patch, MagicMock

@pytest.mark.asyncio
async def test_code_analyzer():
    mock_response = MagicMock()
    mock_response.data = FileAnalysis(
        filename="main.rs",
        purpose="Entry point",
        complexity=3,
        issues=["No error handling"],
        suggestions=["Add Result types"],
    )
    
    with patch.object(file_analyzer, 'run', new_callable=AsyncMock) as mock_run:
        mock_run.return_value = mock_response
        with patch('subprocess.run') as mock_shell:
            mock_shell.return_value = MagicMock(stdout="src/main.rs\n")
            with patch('pathlib.Path.read_text', return_value="fn main() {}"):
                await main()
    
    # Assert... what exactly? stdout? file written?
    # Need more mocking for Path.write_text...
```

---

## Comparison

| Aspect | Cognos | PydanticAI |
|--------|--------|------------|
| **Lines of code** | 68 | 145 |
| **Dependencies** | 0 (just `cognos` binary) | pydantic, pydantic-ai, anthropic SDK, asyncio |
| **Type definitions** | 8 lines | 13 lines (+ imports) |
| **Tool definitions** | Flows — same as any other function | Decorators on agent instances |
| **Agent creation** | None — `think()` is the agent | Explicit `Agent()` objects per task |
| **Structured output** | `format="FileAnalysis"` on any think() | `result_type=` locked to agent instance |
| **Retry/validation** | `validated_think()` — a .cog flow you can read and modify | `retries=3` — black box, can't customize |
| **Error handling** | `try/catch` | `try/except` (same) |
| **Testing** | `cognos test --env mock.json` (one command, zero code) | 20+ lines of mock setup per test |
| **Mock creation** | `cognos trace-to-mock trace.jsonl` (automatic) | Manual mock construction |
| **Shell access** | Sandboxed by default, `--allow-shell` flag | Unrestricted `subprocess.run` |
| **Multi-model** | Change `model=` on any think() call | Create separate Agent per model |
| **Async** | Sync by default (LLM calls are blocking anyway) | Forced async everywhere |
| **Runtime** | Single 5MB binary | Python 3.11+, pip, venv, 50+ transitive deps |
| **Tracing** | `--trace file.jsonl` built-in | OpenTelemetry setup required |

### Key Differences

**1. Agent-per-task vs think-per-call**
PydanticAI creates an `Agent` object per task (file_analyzer, project_summarizer). Cognos just calls `think()` with different parameters. Less ceremony, more flexible.

**2. Testing is night and day**
Cognos: record a trace, derive a mock, replay it. One command. Zero test code.
PydanticAI: AsyncMock, patch decorators, MagicMock, nested context managers. Fragile.

**3. Forced async**
PydanticAI forces `async/await` everywhere because it uses asyncio. But LLM calls take 2-30 seconds — the async overhead buys nothing for sequential agents. Cognos is sync by default, async when you choose it.

**4. Tool registration**
PydanticAI tools are decorators tied to specific agent instances. If two agents need the same tool, you register it twice. Cognos tools are just flows — any think() call can use any flow as a tool.

**5. Observability**
Cognos traces every LLM call, tool call, shell exec, and I/O operation automatically. PydanticAI requires instrumenting with Logfire or OpenTelemetry.
