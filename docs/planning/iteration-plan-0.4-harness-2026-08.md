# 0.4 生产级 Harness — 下一步迭代计划（2026-08）

**范围：** 承接已验收的 Harness 基础（M1–M4、M6、G10、G11），收掉 `closure-plan-2026-06.md` 剩余未收口项（G6 / G8 / G9 / G12 + 发布），使 0.4 生产级防线可对维护者自证闭环。

> 注：G1（微信 CDN live 联调）**不在本计划**——微信 IM bridge 已在历史版本移除（见 `AGENTS.md`：Third-party IM channel bridges are removed），故依赖它的 G1 一并废弃。

**关联 SSOT：**
- 里程碑定义 → [`production-harness-hardening.md`](production-harness-hardening.md)（M0–M8）
- 波次与验收 → [`closure-plan-2026-06.md`](closure-plan-2026-06.md)（Wave 0–4）
- 顶层 backlog → [`../roadmap.md`](../roadmap.md) §4

---

## 0. 基线（2026-08-02 实测）

- **编译**：`cargo check -p anycode-tools -p anycode-dashboard` 通过（含未提交的会话级 plan/todo 持久化 + Workbench Git/Plan 面板改动；仅 2 处 `unused_mut` 警告待清）。
- **已验收**（closure-plan 状态 ☑）：G2 M1 trace SSOT、G3 M2 runtime budget hard-stop、G4 M3 eval 负向 fixture、G5 M4 tool governance metadata、G7 M6 workflow preflight、G10 embeddings CI、G11 Tauri release smoke。
- **已具备但未收口的半成品**：
  - M5 MCP 治理已走 **env 变量**（`ANYCODE_MCP_STRICT` / `ANYCODE_MCP_ALLOWED_TOOLS` / `ANYCODE_MCP_MAX_CALLS_PER_SERVER`），未接入 `config.json`、无 trace 事件、无 Dashboard 面板、缺负向测试。
  - M7 记忆保留已具备 `run_memory_prune(dry_run, apply)`，缺 **provenance 字段**、Workbench Settings UI、memory doctor 联动。

---

## Wave 1 — M5 MCP 治理收口（G6）≈ 1 周

**现状核对（2026-08-02）：** `McpConfigFile` 目前只有 `browser` + `servers`，**无 strict/quota 字段**；`mcp_governance_check` 全走 env（`ANYCODE_MCP_STRICT` / `ANYCODE_MCP_ALLOWED_TOOLS` / `ANYCODE_MCP_MAX_CALLS_PER_SERVER`），无 trace 事件、无 Dashboard 面板。

| 任务 | 主要文件 | 验收 |
|------|----------|------|
| **G6a** config 化：`McpConfigFile` 增 `strict_whitelist` / `max_calls_per_server`；`McpRuntime` 透传；env 回退 | `crates/config/src/schema/types.rs`、`load.rs`、`crates/tools/src/mcp_proxied_tool.rs` | 配置优先、env 回退；无配置不启用 |
| **G6b** `mcp_governance_check` 改读 config（优先）+ env 回退；拒绝/超配额发 **trace 事件**（统一 `events.jsonl` 字段） | `crates/tools/src/mcp_proxied_tool.rs` | Dashboard 读到 `mcp_denied` / `mcp_quota_exceeded` |
| **G6c** Dashboard Settings 暴露 MCP strict / 配额，写回 `config.json` | `crates/dashboard`、`crates/dashboard-ui` | UI 可开关并持久化 |
| **G6d** 负向测试：白名单外工具、超配额、并发计数 | `crates/tools` 单测 + fixture | 缺 metadata 测试 fail（复用 M4 断言风格） |

## Wave 2 — M7–M8 记忆治理（G8）≈ 1 周

**现状核对：** `crates/dashboard/src/memory_ops.rs` 已有 `run_memory_prune(dry_run, apply, older_than_days)`（保护 tag `pin`/`pinned`/`important`/`retain`/`provenance` + 时间窗），但 `RetentionRow` **无 provenance 字段**；无 Settings UI、无 doctor 联动。

| 任务 | 主要文件 | 验收 |
|------|----------|------|
| **G8a** 记忆项补 **provenance** 字段（来源事件 / 会话溯源），向后兼容旧记录 | `crates/core/src/memory_*`、`crates/memory` | 序列化含 provenance，旧记录可读 |
| **G8b** retention 汇总进 **Workbench Settings**，复用 `memory_ops` | `crates/dashboard`、`crates/dashboard-ui` | 与 CLI `memory prune --json` 结果一致 |
| **G8c** provenance 与可回收项进 **memory doctor** 输出 | `crates/dashboard`、doctor 命令 | `doctor` 展示记忆来源与可回收项 |

## Wave 3 — 通道与 cron 决策（G12 / G9）≈ 3–4 天

**现状核对：** `AskUserQuestionHost` trait 已就绪（`ask_user_question_host.rs`）；Telegram inline keyboard 已 MVP（`bootstrap/tg_ask.rs`）；Workbench Web host 走 file IPC（`bootstrap/workbench/workbench_ask.rs`）。Discord 文本回落未做（ADR 008 slice 3）。微信 IM 已移除，不做通道扩展。

| 任务 | 决策 | 验收 |
|------|------|------|
| **G12** AskUserQuestion 通道扩展 | 采纳 **ADR 008 slice 3**：Discord 按钮交互或文本回落；与工具审批互斥 pending | ADR 或 comparison 更新 |
| **G9** NL→cron 定级 | 默认 **A（推荐）**：文档定级 v1 heuristic，Dashboard 提示「规则解析，非 LLM」；`ANYCODE_CRON_NL_LLM=1` 可选 B | comparison + UI 文案 |

## Wave 4 — 文档与发布（收尾）≈ 3 天

- **W4a** 更新 `workbuddy-comparison-2026-06.md`（收口状态、G9 结论）、`../roadmap.md` §2 最近已交付、CHANGELOG（用户可见特性与 breaking）。
- **W4b** `cargo build --release -p anycode-desktop-channel-bridge`（tag 可选）。

---

## 执行顺序与依赖

1. **Wave 1 → Wave 2** 可并行（不同 crate 面：tools/MCP vs memory），均不依赖对方。
2. **Wave 3** 独立决策项，可随时穿插；G9 决策 A 不阻塞其它。
3. **Wave 4** 收尾，须等前 3 波结论。

## 最小 CI 集（收口 PR 必过）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo check -p anycode-tools --features knowledge-embeddings
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-desktop-channel-bridge --features embedded-ui
```

## 风险

| 风险 | 缓解 |
|------|------|
| MCP config 与既有 env 语义冲突 | 配置优先、env 回退；默认不启用 strict |
| memory provenance 破坏旧记录 | 向后兼容反序列化；provenance 可空 |
| 范围膨胀 | 严格按 Wave 1–4 顺序；Wave 2 UI 只做解释性面板，不做新页面风暴 |

*最后更新：2026-08-03 · 依据当前仓库未提交迭代状态与 closure-plan 验收进度；Wave4/CDN(G1) 因微信 bridge 已移除而废弃*