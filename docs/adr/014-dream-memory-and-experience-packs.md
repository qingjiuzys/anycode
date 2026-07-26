# ADR 014: Local dream memory consolidation + offline experience packs

## Status

Accepted (2026-07)

## Context

Earlier roadmaps explicitly **skipped** memory-wiki / dreaming full stacks (see ADR 009 graph-memory spike and 2026-05 OpenClaw sync brief). Product direction has changed:

1. RAG alone cannot form preferences, strategies, or verified recovery loops.
2. Low-model runtime quality must be improved with **structured episodic memory**, **offline teacher experience packs**, and **TaskCompiler** injection — measured on a shared eval baseline.
3. User memory stays local by default; optional cloud sync must be **E2EE** (server stores opaque envelopes only).

## Decision

1. **Dream consolidation is in-scope** as a *local* background pass (scheduler nightly / Desktop idle / Memory Center preview+apply). It is **not** session compaction: compaction owns the context window; dreaming owns cross-session long-term memory.
2. Waking hooks record **structured episode events** (intent / decision / tool trace / acceptance / correction / deliverable), refuse secrets and large raw dumps, and attach **evidence hashes** aligned with `~/.anycode/memory/evidence.jsonl`.
3. Memory V2 metadata (`MemoryKind`, importance, confidence, evidence_hash, TTL, conflicts, pinned) is optional on legacy rows.
4. Runtime recalls **User / Feedback / Project / Reference** with separate budgets; TaskCompiler emits observable `task_spec` / `preferences` / `experience_pack` segments.
5. High models run only in an **offline teacher lab** (`crates/experience` + `scripts/compile-experience-pack.py`). Signed packs may ship with Desktop; teacher keys never enter user runtime.
6. Workflow `depends_on` is executed as a DAG with checkpoints; `parallel_group` / `required_gates` are first-class (layer unlock + gate hints), not schema-only errors.
7. Optional sync: client encrypts with a user master key (file + Keychain mirror on macOS); account-service `/api/v1/memory-sync/*` stores ciphertext, version vectors, and tombstones only. **No server-side dream.**

## Consequences

- Update `docs/roadmap.md` Later section: dreaming is no longer “still skip”.
- Eval promotion gate (`EvalSuiteSummary::meets_promotion_gate`) must pass before enhanced paths become default.
- Memory Center UI surfaces preferences, pending episodes, dream preview/history, and sync mode (`local_only` / `encrypted_sync`).

## Alternatives considered

- Online high-model “teacher at runtime”: rejected (cost, privacy, dependency).
- Server-side dream: rejected while E2EE is the sync model.
- Thousands of always-on agents: rejected; store templates/cards and instantiate 3–7 per task.
