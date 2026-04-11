---
title: 开发与贡献
description: 构建、测试与向 anyCode 贡献代码；工具注册门禁说明。
summary: cargo 常用命令与 registry.rs 顶部 checklist。
read_when:
  - 你要提 PR 或改默认暴露给模型的工具。
---

# 开发与贡献

## 构建

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
```

## 安装到 PATH（可选）

```bash
cargo install --path crates/cli --force
anycode --help
```

## 常用命令

```bash
cargo test
cargo fmt
cargo clippy
```

## 架构阅读顺序（约 5 分钟）

1. **`crates/core/src/traits.rs`** — `Tool`、`LLMClient`、`MemoryStore` 等端口。
2. **`crates/agent/src/runtime/`** — `AgentRuntime` 与工具/LLM 循环（`session.rs`）；编排权威**不是** `Agent::execute`（见仓库 `docs/adr/000-runtime-orchestration.md`）。
3. **`crates/cli/src/bootstrap/runtime.rs`** — CLI/TUI/通道桥共用的 `initialize_runtime` 组装。
4. **[扩展与贡献清单](contributing-extensions)** — registry、catalog、提供商等 checklist。

## 修改默认工具集（门禁）

新增或调整 **默认暴露给模型的工具** 时，必须按 `[crates/tools/src/registry.rs](../../../crates/tools/src/registry.rs)` 文件顶部的 **checklist** 逐项完成（`ins!` 注册、`catalog` 常量、`DEFAULT_TOOL_IDS`、单测等）。若工具涉及写文件、外链、子 Agent、编排等敏感能力，还须把 API 名加入 `[catalog::SECURITY_SENSITIVE_TOOL_IDS](../../../crates/tools/src/catalog.rs)`（CLI `bootstrap` 会据此注册 `SecurityLayer`，勿在 `bootstrap` 再维护平行列表）。

合并前建议至少：

```bash
cargo test -p anycode-tools
cargo test --workspace
```

详见 [架构](architecture.md) 中「Registry」「编排权威与模块边界」。

English: [Development](/guide/development).

## Workspace 说明

Workspace 中的 `anycode-channels` crate 仍在仓库内，**CLI 当前未依赖**，属预留的多通道扩展。

`anycode-memory` 在 workspace `members` 中，且 **CLI 已通过 `bootstrap` 按配置装配**（`memory.backend` 等见 `[cli.md](cli.md)`）；可单独 `cargo test -p anycode-memory` 做库级验证。