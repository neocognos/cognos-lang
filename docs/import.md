# Import

Import flows and types from other `.cog` files.

## Syntax

```cognos
import "path/to/module.cog"
```

Imports must appear at the top of the file, before any type or flow definitions.

## How It Works

- The imported file is parsed and its flows and types are registered
- Imports resolve relative to the importing file's directory
- Imported files can themselves import other files (recursive)
- If two files define the same flow name, the last import wins

## Example

**lib/greet.cog:**
```cognos
flow greet(name: String) -> String:
    return f"Hello, {name}!"
```

**main.cog:**
```cognos
import "lib/greet.cog"

flow main():
    msg = greet("World")
    write(stdout, msg)
```

```bash
$ cognos run main.cog
Hello, World!
```

## Building Libraries

Organize reusable flows into library files:

```
my-agent/
├── lib/
│   ├── shell.cog      # shell(), read_file(), write_file()
│   ├── memory.cog     # remember(), recall()
│   └── http.cog       # fetch(), post()
├── main.cog
```

```cognos
import "lib/shell.cog"
import "lib/memory.cog"

flow main():
    result = shell("ls -la")
    write(stdout, result)
```
