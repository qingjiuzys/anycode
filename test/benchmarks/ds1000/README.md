# DS-1000 adapter

Pinned upstream: `xlang-ai/DS-1000` @ `c4e8f12`.

## Commands

```bash
# Download dataset (hash-pinned)
./download.sh

# Preflight (no model — validates download + Docker image)
./run_adapter.sh "" /tmp/bench-out

# Full eval (models comma-separated, output dir)
./run_adapter.sh agnes,cloud-auto test/results/<run-id>/benchmarks/ds1000
```

## Docker sandbox

Execution uses `--network=none`, read-only root, tmpfs workspace, non-root user.
See `../_common/sandbox.sh` and `../THIRD_PARTY.md`.

## Attribution

See `../THIRD_PARTY.md`.
