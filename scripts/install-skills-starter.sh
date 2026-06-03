#!/usr/bin/env bash
# Copy bundled office skills into ~/.anycode/skills/
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ANYCODE_SKILLS_DIR:-$HOME/.anycode/skills}"
SRC="$ROOT/skills-starter"
mkdir -p "$DEST"
for d in "$SRC"/*/; do
  id="$(basename "$d")"
  if [[ -f "$d/SKILL.md" ]]; then
    mkdir -p "$DEST/$id"
    cp "$d/SKILL.md" "$DEST/$id/SKILL.md"
    if [[ -f "$d/run" ]]; then
      cp "$d/run" "$DEST/$id/run"
      chmod +x "$DEST/$id/run"
    fi
    echo "installed: $id -> $DEST/$id"
  fi
done
echo "Done. Enable skills.enabled in config and run: anycode skills list"
