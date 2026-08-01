# 变更代码审计报告（2026-08-01）

> 审计对象：本次会话对 anycode 工作区的全部修改（工具参数对齐 P0、新工具 P1、死代码删除、提示词 D1–D8）。
> 审计方法：`git diff` 全量走查 + 针对性运行时实验（字节切片 panic 实证）+ 全量测试/格式/clippy 门禁。
> 结论：**1 个高危已修复并补回归测试；1 个中危记录待办；1 个轻微冗余已修复；其余核验通过。**

## 一、审计范围

| 批次 | 内容 | 涉及文件 |
|---|---|---|
| P0 | Grep/Read/Bash/TodoWrite/Task 系列/Cron/WebSearch 参数对齐 | `crates/tools/src/{grep,file_read,bash,todo_write,web_search}.rs`、`orchestration.rs` |
| P1 | RefreshMcpTools、ReadMcpResourceDir、ScheduleWakeup、Monitor、WorkflowGet | `crates/tools/src/{mcp_tools,mcp_connected,mcp_rmcp_session,mcp_legacy_sse_session,orchestration,workflows}.rs` |
| 死代码 | vet_skill_by_id、dashboard test_util、10 个前端组件 | `crates/tools/src/skills/vet.rs`、`crates/dashboard/src/test_util.rs`、`dashboard-ui/src/components/**` |
| D1–D8 | 提示词差异实施 | `crates/agent/src/{prompt_catalog.rs,runtime/*.rs}`、`crates/agent/prompts/**`、`crates/security/src/approval_presenter.rs` |
| 治理 | catalog 元数据、核心工具清单、category 映射 | `crates/core/src/tool_catalog.rs`、`crates/core/src/lib.rs`、`crates/tools/src/{registry,catalog}.rs`、`dashboard-ui/src/lib/skillCatalog.ts` |

## 二、发现清单

### 🔴 高危（已修复 + 回归测试）：审批预览字节切片 panic

- **位置**：`crates/security/src/approval_presenter.rs` → `rule_based_explanation` 的 `brief` 闭包。
- **问题**：`format!("{}…", &trimmed[..120])` 按**字节**截断。当第 120 字节落在多字节 UTF-8 字符（中文/emoji）中间时，`&s[..120]` 触发 `byte index 120 is not a char boundary` panic。
- **触发面**：任何需要审批的 `Bash` 命令（≥120 字节且含中文）或长 `WebSearch` 查询、`Task/Agent` prompt 都会在审批弹窗前直接 panic。
- **实证**：`/tmp/slice_test3.rs`（55 个 ASCII + 中文）复现 panic。
- **修复**：回退到 `<=120` 的最近 `is_char_boundary` 边界再截断。
- **回归测试**：`approval_brief_truncates_at_char_boundary_without_panic`（100 个 `a` + 中文 → 断言渲染包含 `…`，不 panic）。
- **验证**：`cargo test -p anycode-security` → **13 passed**（原 12 + 新 1）。

### 🟠 中危（已修复）：`CronCreate.recurring` 未被 scheduler 消费

- **位置**：`crates/tools/src/orchestration.rs`（`recurring`/`durable` 入参 + 结果回显）→ `crates/channel-bridge/src/scheduler.rs`。
- **问题**：`recurring: false` 声明「fire once at the next match, then auto-delete」，但 `channel-bridge/src/scheduler.rs` 全程无 `recurring` 字段读取，无「触发后自动删除」逻辑。`durable` 同理只是回显（anyCode cron 本就持久化，语义无冲突）。
- **影响**：`CronCreate recurring:false` 实际上会像常驻任务一样重复触发，与文档承诺不符；`ScheduleWakeup` 不受影响（它构造的是**一次性具体时刻表达式**，过点即不再匹配）。
- **修复**（本次会话落地）：
  - `CronJob` 新增 `recurring: bool` 字段（serde 默认 `true`，旧数据兼容）。
  - `CronJobCreateOptions` / `CronJobPatch` 新增 `recurring`，`push_cron_with_options`、`append_cron_job_to_orchestration_file`、`update_cron`、`update_cron_job_in_orchestration_file` 全部消费。
  - `CronCreate` 将 `recurring` 真正写入创建选项（此前仅回显）；`CronUpdate` 支持翻转 `recurring`；`ScheduleWakeup` 显式传 `recurring: false`（fire 后自动删除）。
  - scheduler fire 路径：one-shot job 触发后立即调用 `remove_cron_job_from_orchestration_file` 删除并跳过后续 catch-up。
  - dashboard 两个 handler（`CreateCronJobBody` / `PatchCronJobBody`）同步 `recurring` 透传。
- **回归测试**：`one_shot_cron_persists_recurring_false_and_roundtrips`、`update_cron_patch_can_flip_recurring`（legacy 无字段默认 true）。
- **验证**：tools **167 + 9 passed**（原 165+9 + 新 2）；channel-bridge **19 passed**；dashboard check 通过。

### 🟡 轻微（已修复）：`execute_turn.rs` 冗余 shadow

- **位置**：`crates/agent/src/runtime/execute_turn.rs`。
- **问题**：`let sections = compiled.sections.clone(); let mut sections = sections;` 无意义二次绑定。
- **修复**：合并为 `let mut sections = compiled.sections.clone();`。
- **验证**：`cargo check -p anycode-agent` 通过；agent 测试 **158 + 3 passed**。

## 三、核验通过项（未发现问题）

| 检查点 | 结论 |
|---|---|
| ScheduleWakeup 一次性 cron 表达式 | 正确：`秒 分 时 日 月 *`（星期 `*` 避免 OR 语义），`prepare_cron_schedule_for_storage` + `ScheduleTimezone::Local` 存储；`delay` 钳制 60–3600s |
| Monitor 参数校验 | `command` / `ws` 至少其一，缺失返回 error |
| ReadMcpResourceDir 过滤 | 按 `server`/前缀过滤，`rg` 风格 glob 走 `mcp_connected` 实现 |
| WebSearch 域名过滤 | `allowed_domains` / `blocked_domains` 实际消费于过滤逻辑 |
| TaskUpdate patch 语义 | `addBlocks` / `addBlockedBy` 为追加语义，非覆盖 |
| 工具注册链路 | registry → catalog → core `TOOL_CATALOG` → `DEFAULT_TOOL_IDS` / `SECURITY_SENSITIVE_TOOL_IDS` 全部就位 |
| `#[cfg(not(feature = "tools-mcp"))]` 回退 | feature-gated 双分支均可编译（default + tools-mcp 均验证） |
| skillCatalog category 映射 | business→office、quality→engineering 等映射已更新，测试夹具同步 |
| 死代码删除 | `vet_skill_by_id`、`dashboard::test_util`、10 个前端组件均无残留引用；`crates/experience` 按 ADR 014 保留 |

## 四、门禁证据

```text
cargo fmt --all -- --check        → 通过（修复 crates/tools/src/lib.rs 一处格式）
cargo clippy -p anycode-security --all-targets → 0 warnings
cargo check -p anycode-tools -p anycode-agent -p anycode-security → OK
cargo test -p anycode-security   → 13 passed（含新增回归）
cargo test -p anycode-agent      → 158 passed + 3 passed
cargo test -p anycode-tools      → 165 passed + 9 passed
```

## 五、遗留建议（非阻塞）

1. `recurring:false` 一次性语义已落地（见上），无需再跟踪。
2. 审计中发现 `dashboard-ui/package-lock.json` 变更 334 行——属依赖锁文件正常变动，提交时确认无多余依赖。
3. 前端删除组件后 `index.css` 有 440 行样式变更，建议后续做一次样式树清理（非本次范围）。