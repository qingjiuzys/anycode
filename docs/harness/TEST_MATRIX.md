# Required test matrix

| Area | Included code tests | Real integration still required |
|---|---|---|
| Kernel | tool order, duplicate IDs, capability deny, uncertain output, budget, completion hook, follow-up | existing provider streams, failover, compaction, session and UI cancellation |
| Graph | DAG validation, Partial, gate failure, Human revision, scope/definition mismatch, uncertain recovery, conditional any join | actual NodeExecutor profiles/verifiers, process crash and live storage |
| Subagent | shared budget, separate root rejection, lineage/cancel in context tests | multi-profile write isolation, shared old services migration, native process cleanup |
| Computer | frame/action-bound approval, exclusive lease, unsupported keys, device mismatch | X11 real display, focus race handling; macOS/Windows native drivers |
| 818cloud | issuer/audience/context/origin shape | PostgreSQL/Redis, grant revoke, project membership, device credential lifecycle |
| Frontend | 18 node:test cases | TSX compile, real ReactFlow page, auth callbacks, E2E |
| Installer | 14 unittest checks | full upstream repo + dependency lock + Rust workspace build |

Rust test source being present is not execution evidence. Read root VALIDATION.md before reporting any status.
