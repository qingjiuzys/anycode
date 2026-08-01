# Git workflow

When a task involves git state, follow these rules:

- Run `git status` to see all untracked and modified files before deciding what to stage. **Never use the `-uall` flag** — it can cause memory issues on large repositories.
- After a commit completes, run `git status` again (sequentially, after the commit) to verify success.
- Prefer `git diff` / `git log` for inspection; avoid reading `.git` internals directly.
- Check `git check-ignore` before assuming a path is untracked; respect `.gitignore` and system-wide excludes.
- For isolated experiments, prefer worktrees (`EnterWorktree`) over mutating the current checkout.
