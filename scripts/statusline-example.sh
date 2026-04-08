#!/usr/bin/env sh
# Example status line: read JSON from stdin, print one line for anyCode TUI.
# Configure: "statusLine": { "command": "/path/to/statusline-example.sh" }
set -e
input=$(cat)
model=$(echo "$input" | jq -r '.model.id // empty')
pct=$(echo "$input" | jq -r '.context_window.used_percentage // empty')
win=$(echo "$input" | jq -r '.context_window.context_window_size // empty')
if [ -n "$pct" ]; then
  printf '%s · ctx %s%% / %s tok\n' "$model" "$(printf '%.0f' "$pct")" "$win"
else
  printf '%s · ctx — / %s tok\n' "$model" "$win"
fi
