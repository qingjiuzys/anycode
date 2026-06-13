#!/usr/bin/env bash
# Sync product version from root Cargo.toml [workspace.package] version to npm/Tauri manifests.
# Usage:
#   ./scripts/sync-workspace-version.sh          # write aligned versions
#   ./scripts/sync-workspace-version.sh --check  # exit 1 if any target drifts
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_TOML="$ROOT/Cargo.toml"
UI_PKG="$ROOT/crates/dashboard-ui/package.json"
UI_LOCK="$ROOT/crates/dashboard-ui/package-lock.json"
DESKTOP_CARGO="$ROOT/apps/anycode-desktop/Cargo.toml"
DESKTOP_TAURI="$ROOT/apps/anycode-desktop/tauri.conf.json"

read_workspace_version() {
  local line
  line="$(awk '
    /^\[workspace\.package\]/ { in_ws=1; next }
    /^\[/ { if (in_ws) exit }
    in_ws && /^version[[:space:]]*=/ {
      gsub(/.*version[[:space:]]*=[[:space:]]*"/, "")
      gsub(/".*/, "")
      print
      exit
    }
  ' "$CARGO_TOML")"
  if [[ -z "$line" ]]; then
    echo "failed to read [workspace.package] version from $CARGO_TOML" >&2
    exit 1
  fi
  printf '%s' "$line"
}

read_json_version() {
  python3 - "$1" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    print(json.load(f).get("version", ""))
PY
}

read_desktop_cargo_version() {
  awk '
    /^version[[:space:]]*=/ {
      gsub(/.*version[[:space:]]*=[[:space:]]*"/, "")
      gsub(/".*/, "")
      print
      exit
    }
  ' "$DESKTOP_CARGO"
}

read_npm_lock_version() {
  python3 - "$1" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    data = json.load(f)
print(data.get("version", ""))
PY
}

write_npm_lock_version() {
  python3 - "$1" "$2" <<'PY'
import json, sys
path, version = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as f:
    data = json.load(f)
data["version"] = version
if isinstance(data.get("packages"), dict) and "" in data["packages"]:
    data["packages"][""]["version"] = version
with open(path, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
}

write_json_version() {
  python3 - "$1" "$2" <<'PY'
import json, sys
path, version = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as f:
    data = json.load(f)
data["version"] = version
with open(path, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
}

write_desktop_cargo_version() {
  local version="$1"
  python3 - "$DESKTOP_CARGO" "$version" <<'PY'
import pathlib, re, sys
path = pathlib.Path(sys.argv[1])
version = sys.argv[2]
text = path.read_text(encoding="utf-8")
new_text, n = re.subn(
    r'(?m)^version\s*=\s*"[^"]*"',
    f'version = "{version}"',
    text,
    count=1,
)
if n != 1:
    raise SystemExit(f"expected one version= line in {path}")
path.write_text(new_text, encoding="utf-8")
PY
}

VERSION="$(read_workspace_version)"
CHECK=0
if [[ "${1:-}" == "--check" ]]; then
  CHECK=1
fi

UI_VER="$(read_json_version "$UI_PKG")"
UI_LOCK_VER="$(read_npm_lock_version "$UI_LOCK")"
DESKTOP_CARGO_VER="$(read_desktop_cargo_version)"
DESKTOP_TAURI_VER="$(read_json_version "$DESKTOP_TAURI")"

drift=0
check_one() {
  local label="$1"
  local actual="$2"
  if [[ "$actual" != "$VERSION" ]]; then
    echo "version drift: $label has '$actual', workspace has '$VERSION'" >&2
    drift=1
  fi
}

if [[ "$CHECK" -eq 1 ]]; then
  check_one "crates/dashboard-ui/package.json" "$UI_VER"
  check_one "crates/dashboard-ui/package-lock.json" "$UI_LOCK_VER"
  check_one "apps/anycode-desktop/Cargo.toml" "$DESKTOP_CARGO_VER"
  check_one "apps/anycode-desktop/tauri.conf.json" "$DESKTOP_TAURI_VER"
  if [[ "$drift" -ne 0 ]]; then
    echo "Run ./scripts/sync-workspace-version.sh to align versions." >&2
    exit 1
  fi
  echo "version sync OK ($VERSION)"
  exit 0
fi

write_json_version "$UI_PKG" "$VERSION"
write_npm_lock_version "$UI_LOCK" "$VERSION"
write_desktop_cargo_version "$VERSION"
write_json_version "$DESKTOP_TAURI" "$VERSION"
echo "synced workspace version $VERSION to dashboard-ui, package-lock, anycode-desktop Cargo.toml, tauri.conf.json"
