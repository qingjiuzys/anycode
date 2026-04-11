---
title: 扩展与贡献清单
description: 在 anyCode 工作区中增加默认工具、LLM 提供商、通道与记忆相关代码时应改动的位置。
summary: 面向贡献者的 crate/文件速查表，配合架构页使用。
read_when:
  - 你要增加默认工具、接入新提供商或通道。
---

# 扩展与贡献清单

本文列出常见扩展应修改的**代码位置**。分层与编排权威见 [架构](./architecture)。具体设计决策（例如仅 `AgentRuntime` 负责多轮编排）见仓库内 **`docs/adr/`**（不参与 VitePress 构建）。

## 新增默认工具

1. 在 `crates/tools` 中实现 `anycode_core::traits::Tool`。
2. 在 `crates/tools/src/registry.rs` 注册 —— 遵守文件顶部 **checklist**（`ins!`、`DEFAULT_TOOL_IDS`、测试等）。
3. 若工具涉及写盘、子 Agent、网络等敏感能力，在 `crates/tools/src/catalog.rs` 的 `SECURITY_SENSITIVE_TOOL_IDS` 中登记，以便 **`bootstrap`** 统一注册 `SecurityLayer` 策略。
4. 运行 `cargo test -p anycode-tools` 与 `cargo test --workspace`。

## 新 LLM 提供商或传输方式

1. 在 `crates/llm` 实现 `LLMClient`（可参考 `crates/llm/src/providers/`）。
2. 在 `transport_for_provider_id` / `MultiProviderLlmClient` 等处接入路由（`lib.rs`、`provider_catalog.rs` 等）。
3. 在 `crates/llm` 补充测试。

## 新通道（微信 / Web 等）

1. 在 `crates/channels` 实现 `ChannelHandler`。
2. 主 CLI 未必已依赖 `channels`；在用户可见入口就绪时于 **`crates/cli` 组合根**接线。

## 记忆后端与 pipeline

- **file / hybrid / noop**：由 CLI `bootstrap` 的 `build_memory_layer` 配置（`crates/cli/src/bootstrap/mod.rs`）。
- **pipeline**（向量与可选 embedding）：领域类型见 `crates/core/src/memory_pipeline.rs`，实现见 `crates/memory`。详见 [ADR 001](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/001-memory-pipeline-and-store.md)。

## 速查表

| 目标 | 优先打开的文件 |
|------|----------------|
| 工具注册 | `crates/tools/src/registry.rs` |
| 目录与敏感工具 ID | `crates/tools/src/catalog.rs` |
| 运行时组装 | `crates/cli/src/bootstrap/runtime.rs` |
| Agent 多轮循环 | `crates/agent/src/runtime/session.rs`、`mod.rs` |
