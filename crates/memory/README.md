# anycode-memory

Memory storage and retrieval support for anycode.

## Responsibilities

- File-backed memory persistence.
- Optional embedding/vector backends behind crate features.
- Memory pipeline helpers used by the runtime composition root.

Keep cross-crate memory contracts in `anycode-core`; keep storage and backend implementation details here.
