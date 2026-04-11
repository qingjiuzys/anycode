---
title: Development
description: Build, test, and contribute to anyCode — fmt, clippy, and tool registry checklist.
summary: Workspace commands and where to register new default tools safely.
read_when:
  - You open a PR or add a tool to the default registry.
---

# Development

## Build

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
```

## Install to PATH (optional)

```bash
cargo install --path crates/cli --force
anycode --help
```

## Common commands

```bash
cargo test
cargo fmt
cargo clippy
```

## Architecture reading order (~5 minutes)

1. **`crates/core/src/traits.rs`** — `Tool`, `LLMClient`, `MemoryStore`, and other ports.
2. **`crates/agent/src/runtime/`** — `AgentRuntime` and the tool/LLM loop (`session.rs`); orchestration authority is **not** `Agent::execute` (see `docs/adr/000-runtime-orchestration.md` in the repo).
3. **`crates/cli/src/bootstrap/runtime.rs`** — builds the shared runtime for CLI/TUI/channel bridges (`initialize_runtime`).
4. **[Contributing extensions](./contributing-extensions)** — checklists for registry, catalog, and providers.

## Changing the default tool surface

When adding or changing **tools exposed to the model by default**, follow the **checklist** at the top of **`crates/tools/src/registry.rs`** (`ins!` registration, `catalog` constants, **`DEFAULT_TOOL_IDS`**, tests, etc.). If a tool can write files, hit the network, spawn sub-agents, or similar, also add its API name to **`catalog::SECURITY_SENSITIVE_TOOL_IDS`** in **`crates/tools/src/catalog.rs`** — the CLI **`bootstrap`** registers **`SecurityLayer`** from this; do **not** maintain a parallel list only in **`bootstrap`**.

Before merging, run at least:

```bash
cargo test -p anycode-tools
cargo test --workspace
```

See [Architecture](./architecture) for registry boundaries and orchestration.

## Workspace notes

- **`anycode-channels`** remains in the repo; the **CLI does not depend on it yet** — reserved for multi-channel expansion.  
- **`anycode-memory`** is a workspace member and is **wired through CLI `bootstrap`** when configured (`memory.backend`, etc.); **`cargo test -p anycode-memory`** validates the library.

## Related

- Root **README** — full contributor workflow  
- [Roadmap](./roadmap) — staged tool work  
