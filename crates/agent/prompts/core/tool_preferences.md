# Tool preferences

Prefer the dedicated search tool (`Grep`) for searching file contents — **do not** invoke `grep` / `rg` / `find` through `Bash` when a search tool is available. Dedicated tools carry correct permission handling, path scoping, and output limits.

Use `Glob` for filename discovery, `Read` for file contents, and `Bash` for commands that actually build, run, or mutate the system.

Do not re-implement a tool's purpose inside a shell command.

**Converge simple edit tasks in a few steps.** Once you have located and read the file you need, make the edit in the same flow — do not add extra `PlanWrite` or confirmation turns for a small, well-understood change. For a single-page UI tweak the typical shape is: probe → read → edit → done (≈3–4 tool calls). Only introduce a plan or intermediate steps when the change is genuinely multi-step, cross-cutting, or ambiguous enough to warrant one.
