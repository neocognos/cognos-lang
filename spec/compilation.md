# Cognos Compilation Specification

## Overview

Cognos source files (`.cog`) compile to the same internal `StageDef` representation that the Neocognos kernel already understands and executes. This design choice provides several key benefits:

1. **No kernel changes required** - The existing Neocognos runtime works unchanged
2. **Feature parity** - Cognos can express anything that YAML workflows can
3. **Interoperability** - Cognos and YAML workflows can call each other
4. **Proven runtime** - Leverages the mature, battle-tested kernel

## Compilation Pipeline

```
┌─────────────┐    ┌────────────┐    ┌─────────────┐    ┌──────────────┐
│ .cog source │ => │   Parser   │ => │   Compiler  │ => │  StageDef    │
│    files    │    │    (AST)   │    │ (semantic)  │    │ (JSON/binary)│
└─────────────┘    └────────────┘    └─────────────┘    └──────────────┘
                                                              │
                                                              v
                                                    ┌──────────────┐
                                                    │  Neocognos   │
                                                    │   Kernel     │
                                                    │  (Runtime)   │
                                                    └──────────────┘
```

### Phase 1: Parsing
- Lexical analysis breaks source into tokens
- Parser builds Abstract Syntax Tree (AST) using the PEG grammar
- Syntax errors are caught and reported with precise location information

### Phase 2: Semantic Analysis
- Type checking ensures all expressions are well-typed
- Symbol resolution links identifiers to their definitions
- Flow dependency analysis enables proper compilation order
- Semantic errors are caught and reported

### Phase 3: Code Generation
- AST is lowered to `StageDef` representation
- Cognos constructs map to equivalent kernel primitives
- Optimization passes may be applied

## Cognos to Kernel Mapping

### Flow Definitions
```cognos
flow example_flow(param: Text) -> Text:
    result = think(param)
    return result
```

Compiles to a `StageDef` with:
- Input schema defining the parameter structure
- Step definitions for each statement
- Output schema defining the return type

### Built-in Functions

| Cognos Function | Kernel Implementation |
|----------------|----------------------|
| `think()` | LLM inference step with prompt construction |
| `act()` | Tool execution step |
| `receive()` | Input binding from runtime context |
| `emit()` | Output emission to runtime context |
| `run()` | Shell command execution step |
| `read_file()` | File system read operation |
| `write_file()` | File system write operation |
| `remember()` | Memory storage operation |
| `recall()` | Memory retrieval operation |
| `log()` | Debug output step |

### Control Flow

#### Conditionals
```cognos
if condition:
    statement1
else:
    statement2
```

Compiles to conditional step execution in the kernel with proper branching logic.

#### Loops
```cognos
loop max=10:
    statement
```

Compiles to bounded loop construct with automatic iteration tracking and termination.

#### Parallel Execution
```cognos
a, b = parallel:
    operation1()
    operation2()
```

Compiles to parallel execution group with result collection and synchronization.

### Type System Integration

Cognos types map to kernel type schemas:

- **Primitive types** (`Text`, `Bool`, `Int`, `Float`) map to corresponding JSON schema primitives
- **Container types** (`List[T]`, `Map[K,V]`) map to JSON schema arrays and objects
- **Custom types** compile to structured JSON schemas with field validation
- **Optional types** use JSON schema nullable properties

### Error Handling

```cognos
try:
    risky_operation()
catch error:
    handle_error(error)
```

Compiles to kernel error handling mechanisms with proper exception propagation and recovery.

## Compilation Artifacts

### Input Format
- Source files: `*.cog`
- Configuration: `cognos.toml` (optional)

### Output Format
- Primary output: `StageDef` JSON/binary
- Debug symbols: Source maps for runtime debugging
- Type information: Schema definitions for validation

### Example Output Structure
```json
{
  "stage_id": "example_flow",
  "input_schema": {
    "type": "object",
    "properties": {
      "param": {"type": "string"}
    },
    "required": ["param"]
  },
  "steps": [
    {
      "id": "step_1",
      "type": "llm_call",
      "config": {
        "prompt_template": "{{param}}",
        "system_prompt": "",
        "tools": []
      }
    }
  ],
  "output_schema": {
    "type": "string"
  }
}
```

## Implementation Details

### Compiler Architecture

The Cognos compiler is implemented in Rust and consists of several modules:

```
cognos-compiler/
├── src/
│   ├── lexer.rs          # Tokenization
│   ├── parser.rs         # AST construction
│   ├── ast.rs            # AST node definitions
│   ├── analyzer.rs       # Semantic analysis
│   ├── typechecker.rs    # Type system
│   ├── codegen.rs        # StageDef generation
│   ├── error.rs          # Error handling
│   └── main.rs           # CLI interface
├── tests/                # Test suite
└── Cargo.toml
```

### CLI Interface

```bash
# Compile a single file
cognos compile example.cog -o example.stage.json

# Compile a project
cognos build

# Type check only
cognos check *.cog

# Watch mode for development
cognos watch src/
```

### Configuration

Optional `cognos.toml` for project-level settings:

```toml
[project]
name = "my-workflow"
version = "0.1.0"

[compiler]
target = "neocognos-v1"
optimization_level = 1

[output]
format = "json"  # or "binary"
compress = true
include_debug = false
```

## Future Enhancements

### LSP Server
A Language Server Protocol implementation will provide:
- Real-time syntax highlighting
- Type information on hover
- Auto-completion
- Error squiggles
- Refactoring support

### Tree-sitter Grammar
A tree-sitter grammar will enable:
- Syntax highlighting in editors
- Structural navigation
- Code folding
- AST-based tooling

### Advanced Optimizations
Future compiler versions may include:
- Dead code elimination
- Common subexpression elimination
- Flow inlining for simple cases
- Parallel execution optimization

### Debugging Support
Integration with the kernel debugger for:
- Breakpoint support
- Variable inspection
- Step-through execution
- Flow call stack traces

## Backward Compatibility

The compilation target (`StageDef`) is designed to be stable across kernel versions. This ensures that:
- Compiled Cognos workflows continue working as the kernel evolves
- Mixed deployments (Cognos + YAML) are fully supported
- Migration from YAML to Cognos can be gradual

## Alternative Syntaxes

While Cognos provides a clean imperative syntax, the kernel's `StageDef` format also supports other potential frontends:
- YAML workflows (existing)
- Functional language frontends
- Visual workflow builders
- Domain-specific languages

This flexibility allows teams to choose the syntax that best fits their workflow and expertise while sharing the same robust execution engine.