# CodeContests (stratified) adapter

Pinned upstream: `google-deepmind/code_contests` @ `f1a9d3e`.

## Commands

```bash
# Download dataset (hash-pinned)
./download.sh

# Preflight (no model — validates download + Docker image)
./run_adapter.sh "" /tmp/bench-out

# Full eval (models comma-separated, output dir)
./run_adapter.sh agnes,cloud-auto test/results/<run-id>/benchmarks/codecontests
```

## Docker sandbox

Execution uses `--network=none`, read-only root, tmpfs workspace, non-root user.
See `../_common/sandbox.sh` and `../THIRD_PARTY.md`.

## Attribution

See `../THIRD_PARTY.md`.
