---
title: 记忆系统说明
description: anyCode 记忆与 OpenClaw 式 memory 扩展的对照备忘。
summary: 当前后端与 scope；后续可对标的改进清单。
read_when:
  - 对比 OpenClaw / Claude Code 的记忆行为。
---

# 记忆系统说明

## anyCode 现状

- **后端**：`memory.backend` 支持 `file` / `hybrid` / `noop`，以及 **`pipeline`**（归根通道：虚态缓冲 → 强化 → 热层 Sled → 可选向量；实现见 `anycode_memory::RootReturnMemoryPipeline`）。别名：`layered`、`guigen`。
- **旧版 Markdown**：在 `pipeline` 且 `memory.pipeline.merge_legacy_file_recall` 为 true（默认）时，记忆根目录下既有 `*.md` 会以**只读**方式并入召回，与热层并行。
- **作用域**：项目 / 用户记忆经 `anycode_memory`；pipeline 在晋升前多一层「前语义」片段。
- **自动保存**：由 `memory.auto_save` 与任务成功后的 runtime 钩子控制。使用 **`pipeline`** 时，自动保存走 **ingest**（先入缓冲）；多次触碰后才写入热层。

可选 `memory.pipeline` 字段包括：`buffer_ttl_secs`、`max_buffer_fragments`、`promote_touch_threshold`、`reinforce_on_recall_match`、`merge_legacy_file_recall`、`buffer_wal_enabled`、`buffer_wal_fsync_every_n`、`hook_after_tool_result`、`hook_after_agent_turn`、`hook_max_bytes`、`hook_tool_deny_prefixes`、`embedding_enabled`、`embedding_model`、`embedding_base_url`、`embedding_provider`、`embedding_local_cache_dir`。

- **WAL**：默认 `buffer_wal_enabled` 为 true 时，虚态缓冲追加写入热层旁 `*.pipeline.buffer.wal`（JSONL），启动时重放；除按 `buffer_wal_fsync_every_n` 定期刷盘外，Telegram/Discord/内置调度器/微信桥单次任务、以及 `run`/编排触发的工作单元结束后也会刷盘，进程正常退出、管线释放时还会再 `fsync` 一次。
- **向量**：`embedding_enabled`、非空 `embedding_model`，或 **`embedding_provider` 为 `local`** 时，向量写入 `*.pipeline.vec.sled`，余弦检索。  
  - **`embedding_provider`**：`http`（默认）走 OpenAI 兼容 `…/embeddings`，复用 `llm.api_key`，可用 `embedding_base_url` 换主机。  
  - **`local`**：通过 [FastEmbed](https://github.com/Anush008/fastembed-rs) 在本地跑 ONNX（默认 `all-MiniLM-L6-v2`，首次自动拉取）。需使用 **`cargo build -p anycode --features embedding-local`** 构建。可选 `embedding_local_cache_dir` 指定模型缓存目录（默认可为 `~/.cache/fastembed` 一带）。
- **导入**：`anycode memory import [--dry-run] [--limit N]` 将 `memory.path` 下 legacy Markdown 批量写入 pipeline 热层（需 `memory.backend: pipeline`）。

## 与 OpenClaw 对标（研究 backlog）

OpenClaw 将记忆做成**扩展**，保留策略与召回路径独立。可对标项：

1. **写入时机**：仅在任务成功 vs 工具写入 vs 显式命令。
2. **检索**：关键词 vs 向量 / 混合；项目隔离保证。
3. **与压缩关系**：`/compact` 与会话自动压缩后记忆如何保留。

建议在 issue 里程碑跟踪，而非在 CLI 二进制内复制 OpenClaw 全部实现。

## 相关

- [架构](./architecture)  
- [配置与安全](./config-security)  
