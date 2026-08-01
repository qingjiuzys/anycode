# anyCode 死代码审计报告

**日期**: 2026-08-01
**范围**: `crates/*`（Rust）+ `crates/dashboard-ui`（前端 TSX）
**方法**: `cargo clippy --workspace --all-targets` dead_code 告警 + 全仓符号引用计数（排除 pub use / 定义行 / 类型签名误报）+ 前端 import 图分析

---

## 结论速览

| 级别 | 数量 | 说明 |
| --- | --- | --- |
| 整 crate 无依赖 | 1 | `crates/experience` |
| Rust 明确死代码（clippy） | 1 处 | dashboard `test_util` 的锁 |
| Rust 无内部调用者 pub fn | 1 | `vet_skill_by_id` |
| 前端无任何 import 组件 | 10 | 见下 |

---

## 一、整 crate 无依赖：`crates/experience`

- workspace member（`Cargo.toml` 第 11 行），但**没有任何 crate 依赖它**：
  - 全仓 `Cargo.toml` 中无 `anycode-experience` 依赖（除自身）
  - `cargo tree -i anycode-experience` 只回显自身
  - 源码无 `anycode_experience` / `experience::` 引用（dashboard/bootstrap/agent/channel-bridge 均无）
  - release 脚本（`scripts/*.sh`、Makefile）无引用
- 内容：`src/lib.rs` 15 个 pub fn（`distill_card`、`filter_validated`、`sign_pack_hmac_like`、`build_pack_from_trajectories` 等），仅依赖 anycode-core；含测试。
- 判定：**候选整 crate 移除**。若有意保留为离线工具库，建议加 `exclude` 出 workspace 或补 feature 门控；当前它会被 workspace 构建但无人消费。

## 二、clippy 明确报 dead_code：dashboard test_util

`crates/dashboard/src/test_util.rs`：
- `static STATE_DIR_TEST_LOCK: Mutex<()>` —— never used
- `pub fn lock_state_dir_env()` —— never used

dashboard 内部无任何 `crate::test_util::` 引用；dashboard-ipc 有自己的 `test_util.rs`（同名函数，被 question_ipc/cancel_ipc/approval_ipc 测试使用，是活代码）。dashboard 这份是**遗留副本**，判定可删除。

## 三、Rust pub fn 无内部调用者

| 符号 | 位置 | 证据 |
| --- | --- | --- |
| `vet_skill_by_id` | `crates/tools/src/skills/vet.rs:80` | 仅 `pub use` 导出（lib.rs / skills/mod.rs）；无任何调用点。`vet_skill_dir` 是活代码（install.rs 3 处调用），二者是不同符号 |

> 已排除的误报（均有真实调用）：`merge_agent_type_tool_denies`（agent×3）、`known_cron_tool_profiles`/`known_cron_failure_destinations`（orchestration×4）、`scan_listed_tools`（registry）、`scan_tool_entry`（mcp_tool_scan 内部）、`default_skill_roots`（bootstrap）、`tool_catalog`（dashboard governance）、`attach_vision_images`（agent/dashboard）、compact 系列（`inject_file_read_snippets`/`apply_microcompact`/`summarization_start_index`/`collect_from_session`）、`skill_resolved_marker`（execute_task/execute_turn）、`relevant_memories_context_section`（runtime/mod）、knowledge_vectors 全部函数（dashboard project_knowledge/workbench_doctor）。

## 四、前端无任何 import 的组件（10 个）

| 组件 | 说明 |
| --- | --- |
| `AppearanceMenu.tsx` | 无 import / 无路由 / 无动态加载 |
| `AutomationCreatePanel.tsx` | 同上 |
| `ExecutionProgressBar.tsx` | 同上 |
| `SidebarWorkspaceCard.tsx` | 同上 |
| `ThemeToggle.tsx` | 同上 |
| `TopbarNewMenu.tsx` | 同上 |
| `ui/FilterBar.tsx` | 同上 |
| `chat/AgentWorkLog.tsx` | 同上 |
| `chat/AgentPhaseSection.tsx` | 同上 |
| `service/ServiceCloudLogin.tsx` | 同上 |

- 无 barrel 导出（`components/` 无 `index.ts`）；无 `import()` 动态加载；无 e2e / index.html 引用。
- 排除项：`ConversationArtifactsPanel.tsx` 是 re-export 别名（被 `ConversationInspectorPanel.tsx` 引用，需确认后者）；`TurnPhaseBanner` 仅出现在注释中。
- 这些多为侧边栏/菜单重构后的遗留旧组件（`SidebarWorkspaceCard`、`TopbarNewMenu`、`AppearanceMenu` 对应新 `session/SessionSidebar.tsx` 等）。

---

## 五、处理建议

1. **已安全删除**（2026-08-01 执行，`cargo check`/`cargo test`/`npm run build` 全绿）：
   - `crates/dashboard/src/test_util.rs` 整文件 + `lib.rs` 的 `#[cfg(test)] mod test_util;`（dashboard-ipc 有自己的活副本）
   - `vet_skill_by_id`（`crates/tools/src/skills/vet.rs`）及其两处 `pub use` 导出，顺带清理残留 `PathBuf` 导入
   - 前端 10 个无引用组件文件：`AppearanceMenu`、`AutomationCreatePanel`、`ExecutionProgressBar`、`SidebarWorkspaceCard`、`ThemeToggle`、`TopbarNewMenu`、`ui/FilterBar`、`chat/AgentWorkLog`、`chat/AgentPhaseSection`、`service/ServiceCloudLogin`
2. **保留（架构设计内）**：
   - `crates/experience` 整 crate：ADR 014 明确将其设计为「离线教师实验室」（`crates/experience` + `scripts/compile-experience-pack.py`），teacher keys 不进入用户运行时。虽当前无 Rust 依赖方，但属有意保留，删除会破坏 ADR 014 契约，**不删**。
3. 建议后续在 CI 增加 `cargo clippy --workspace --all-targets -- -D warnings` 或对 dashboard 开启 `#[warn(dead_code)]` 审查，防止回归。

---

## 附：验证命令

```bash
cargo clippy --workspace --all-targets 2>&1 | grep -E "never used"
cargo tree -i anycode-experience
grep -rn "anycode-experience" --include="Cargo.toml" .
for f in crates/dashboard-ui/src/components/*.tsx; do ... import 计数 ...; done
```