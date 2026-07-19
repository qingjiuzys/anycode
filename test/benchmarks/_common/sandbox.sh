#!/usr/bin/env bash
# Shared Docker sandbox flags for executing untrusted generated code.
# Source from adapter run_adapter.sh — do not execute directly.
set -euo pipefail

ANYCODE_BENCH_SANDBOX_NET="${ANYCODE_BENCH_SANDBOX_NET:-none}"
ANYCODE_BENCH_SANDBOX_MEMORY="${ANYCODE_BENCH_SANDBOX_MEMORY:-4g}"
ANYCODE_BENCH_SANDBOX_CPUS="${ANYCODE_BENCH_SANDBOX_CPUS:-2}"
ANYCODE_BENCH_SANDBOX_PIDS="${ANYCODE_BENCH_SANDBOX_PIDS:-256}"
ANYCODE_BENCH_SANDBOX_UID="${ANYCODE_BENCH_SANDBOX_UID:-1000:1000}"

anycode_bench_sandbox_flags() {
  printf '%s\n' \
    --network="${ANYCODE_BENCH_SANDBOX_NET}" \
    --read-only \
    --tmpfs /tmp:rw,noexec,nosuid,size=512m \
    --tmpfs /workspace:rw,noexec,nosuid,size=1g \
    --memory="${ANYCODE_BENCH_SANDBOX_MEMORY}" \
    --cpus="${ANYCODE_BENCH_SANDBOX_CPUS}" \
    --pids-limit="${ANYCODE_BENCH_SANDBOX_PIDS}" \
    --user "${ANYCODE_BENCH_SANDBOX_UID}" \
    --cap-drop=ALL \
    --security-opt=no-new-privileges
}

anycode_bench_run_sandboxed() {
  local image="$1"
  shift
  # shellcheck disable=SC2048,SC2086
  docker run --rm $(anycode_bench_sandbox_flags) "$image" "$@"
}
