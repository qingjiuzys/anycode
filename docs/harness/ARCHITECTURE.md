# Harness architecture and constraints

See RESEARCH.zh-CN.md for source-grounded rationale and CURSOR_START.md for migration order.

## Dependency direction

harness-core has no anycode-agent/tools/dashboard/cloud dependency. harness-extensions depends on core. harness-cloud818 depends on core. harness-host depends on core + extensions + existing anycode-core. anycode-agent optionally depends on harness-host/core under harness-v1. There is no circular Cargo dependency.

## Host trust

RunContext must originate from a trusted host after account/project authorization, or explicit local-only setup. Scope deserialization is not proof of identity. Capabilities are exact names. An injected Host can always act with its native process privileges, so these interfaces are not a replacement for OS isolation.

The runtime bridge adds HarnessBoundary and still calls execute_tool_call, preserving security/gating/approval/audit. No default enterprise boundary is shipped. ReadOnlyPilotBoundary rejects enterprise scope and permits only FileRead.

## Resource contracts

Computer input uses device + scope + run + frame + target + arguments + one-use approval. File stores require a private, immutable host-owned directory. Canonical paths do not eliminate TOCTOU when an attacker can rewrite parent directories. Run subprocess helper only in trusted workers; killing direct children is not process-tree containment.

## Persistence

Journal: JSONL, sequence/hash chain, fsync, cross-process writer lease, no silent tail repair. Preview bus can drop ephemeral notifications, not durable facts. UI must resync from authenticated journal projections and never use a preview as execution truth.

Checkpoint: explicit fresh versus resume, definition/scope/run binding, in-process operation lock, cross-process file lease, atomic file replacement. It is not a distributed run store. Crashed Running nodes become Uncertain, never automatically re-executed.

Budget: all descendants share reservation pool; unknown sent usage is conservatively charged. Restoring snapshots with reservations charges those reservations. This is token budgeting, not monetary accounting or completed multi-worker budget reconciliation.

## Compatibility policy

No Pi runtime dependency; no claim of Pi plugin ABI/API compatibility. No LangGraph checkpoint compatibility. DAG v1 rejects cycles. Existing FileRead and model DTOs are reused. Streaming response-chain optimization is deliberately not synthesized; full native history is sent when provider response-id prefix semantics are not available. Existing failover, compaction and memory behavior remain a migration requirement.
