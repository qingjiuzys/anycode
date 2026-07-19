#!/usr/bin/env bash
# Resolve anyCode packaging target: local loopback vs hosted cloud (anycode.work).
#
# Usage (from other scripts):
#   source "$ROOT/scripts/lib/build-target.sh"
#   anycode_apply_build_target_exports
#
# Explicit override:
#   ANYCODE_BUILD_TARGET=local|cloud
#
# Implicit defaults:
#   ANYCODE_DESKTOP_LOCAL_RELEASE=1  → local (fast local DMG / install-desktop-macos --build)
#   otherwise                        → cloud (signed release, CI, account image)
set -euo pipefail

ANYCODE_CLOUD_ORIGIN="${ANYCODE_CLOUD_ORIGIN:-https://anycode.work}"

ANYCODE_LOCAL_ACCOUNT_API_PORT="${ANYCODE_LOCAL_ACCOUNT_API_PORT:-43200}"
ANYCODE_LOCAL_ACCOUNT_PORTAL_PORT="${ANYCODE_LOCAL_ACCOUNT_PORTAL_PORT:-43201}"
ANYCODE_LOCAL_MODEL_GATEWAY_PORT="${ANYCODE_LOCAL_MODEL_GATEWAY_PORT:-43210}"

anycode_resolve_build_target() {
  if [[ -n "${ANYCODE_BUILD_TARGET:-}" ]]; then
    case "${ANYCODE_BUILD_TARGET}" in
      local|cloud) echo "${ANYCODE_BUILD_TARGET}"; return 0 ;;
      *)
        echo "ANYCODE_BUILD_TARGET must be 'local' or 'cloud' (got: ${ANYCODE_BUILD_TARGET})" >&2
        return 1
        ;;
    esac
  fi
  if [[ "${ANYCODE_DESKTOP_LOCAL_RELEASE:-}" == "1" ]]; then
    echo "local"
    return 0
  fi
  echo "cloud"
}

anycode_local_account_api_url() {
  echo "http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_API_PORT}"
}

anycode_local_account_portal_url() {
  echo "http://127.0.0.1:${ANYCODE_LOCAL_ACCOUNT_PORTAL_PORT}"
}

anycode_local_model_gateway_url() {
  echo "http://127.0.0.1:${ANYCODE_LOCAL_MODEL_GATEWAY_PORT}"
}

anycode_apply_build_target_exports() {
  local target
  target="$(anycode_resolve_build_target)"
  export ANYCODE_BUILD_TARGET="${target}"

  if [[ "${target}" == "local" ]]; then
    export ANYCODE_ACCOUNT_API_URL="$(anycode_local_account_api_url)"
    export ANYCODE_ACCOUNT_PORTAL_URL="$(anycode_local_account_portal_url)"
    export ANYCODE_MODEL_GATEWAY_URL="$(anycode_local_model_gateway_url)"
    export ACCOUNT_PUBLIC_URL="${ACCOUNT_PUBLIC_URL:-${ANYCODE_ACCOUNT_API_URL}}"
    export ACCOUNT_PORTAL_URL="${ACCOUNT_PORTAL_URL:-${ANYCODE_ACCOUNT_PORTAL_URL}}"
    export VITE_ACCOUNT_API_URL="${ANYCODE_ACCOUNT_API_URL}"
    export VITE_ACCOUNT_PORTAL_URL="${ANYCODE_ACCOUNT_PORTAL_URL}"
  else
    export ANYCODE_ACCOUNT_API_URL="${ANYCODE_ACCOUNT_API_URL:-${ANYCODE_CLOUD_ORIGIN}}"
    export ANYCODE_ACCOUNT_PORTAL_URL="${ANYCODE_ACCOUNT_PORTAL_URL:-${ANYCODE_CLOUD_ORIGIN}}"
    export ANYCODE_MODEL_GATEWAY_URL="${ANYCODE_MODEL_GATEWAY_URL:-${ANYCODE_CLOUD_ORIGIN}}"
    export ACCOUNT_PUBLIC_URL="${ACCOUNT_PUBLIC_URL:-${ANYCODE_CLOUD_ORIGIN}}"
    export ACCOUNT_PORTAL_URL="${ACCOUNT_PORTAL_URL:-${ANYCODE_CLOUD_ORIGIN}}"
    unset VITE_ACCOUNT_API_URL VITE_ACCOUNT_PORTAL_URL
  fi
}

anycode_write_account_endpoints_manifest() {
  local out="$1"
  local target api portal gateway
  target="$(anycode_resolve_build_target)"
  anycode_apply_build_target_exports
  api="${ANYCODE_ACCOUNT_API_URL}"
  portal="${ANYCODE_ACCOUNT_PORTAL_URL}"
  gateway="${ANYCODE_MODEL_GATEWAY_URL}"
  mkdir -p "$(dirname "${out}")"
  python3 - "${out}" "${target}" "${api}" "${portal}" "${gateway}" <<'PY'
import json, sys
path, target, api, portal, gateway = sys.argv[1:6]
payload = {
    "target": target,
    "account_api_url": api,
    "account_portal_url": portal,
    "model_gateway_url": gateway,
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2)
    f.write("\n")
PY
}

anycode_print_build_target_summary() {
  anycode_apply_build_target_exports
  echo "anyCode build target: ${ANYCODE_BUILD_TARGET}"
  echo "  account API:    ${ANYCODE_ACCOUNT_API_URL}"
  echo "  account portal: ${ANYCODE_ACCOUNT_PORTAL_URL}"
  echo "  model gateway:  ${ANYCODE_MODEL_GATEWAY_URL}"
}
