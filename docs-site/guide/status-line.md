# Status line (HUD)

The fullscreen TUI can show a **bottom status line** (above the prompt), similar to Claude Code’s `statusLine` setting.

## Configuration

In `~/.anycode/config.json`, add a `statusLine` object:

```json
{
  "statusLine": {
    "command": "jq -r '[.model.id, .context_window.used_percentage // empty] | @tsv' | paste -sd ' ' -",
    "timeout_ms": 5000,
    "padding": 0,
    "show_builtin": false
  }
}
```

- **`command`**: passed to `sh -c`. The process receives **one JSON document on stdin** (see below). Only the **first line** of stdout is shown (ANSI sequences are stripped).
- **`timeout_ms`**: defaults to `5000`. The command is killed on timeout.
- **`padding`**: left padding (columns) before the text.
- **`show_builtin`**: if `true` and **`command` is omitted**, a built-in line is shown: model id and approximate context usage vs the configured window.

**Security:** `command` runs with your user permissions. Treat it like any shell script in your config.

## JSON schema (subset)

Print a sample payload (pretty-printed) for your current config and working directory:

```bash
anycode statusline print-schema
```

Fields include: `version`, `session_id`, `cwd`, `model.id`, `model.display_name`, `workspace.current_dir`, `workspace.project_dir`, `context_window.context_window_size`, token counts and optional `used_percentage` / `remaining_percentage`.

## Example script

See `scripts/statusline-example.sh` in the repository for a `jq`-based example.

