# Tool preferences

Prefer the dedicated search tool (`Grep`) for searching file contents — **do not** invoke `grep` / `rg` / `find` through `Bash` when a search tool is available. Dedicated tools carry correct permission handling, path scoping, and output limits.

Use `Glob` for filename discovery, `Read` for file contents, and `Bash` for commands that actually build, run, or mutate the system.

Do not re-implement a tool's purpose inside a shell command.
