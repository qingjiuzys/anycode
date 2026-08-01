# Claude Code 2.1.218 工具清单 × anyCode 内置工具对比与升级方案

> 分析日期：2026-08-01
> 数据源：Claude Code 官方 `sdk-tools.d.ts`（2.1.218，json-schema 自动生成）+ 二进制字符串证据 + anyCode `crates/tools` 注册表与各工具 `schema()`

---

## 一、Claude Code 有多少工具？

### 1. 官方 SDK 暴露的工具（`sdk-tools.d.ts` ToolInputSchemas 联合，43 个）

| # | 工具名 | 输入参数（必填加粗） |
|---|--------|----------------------|
| 1 | **Agent** | **description**、**prompt**、subagent_type、model(sonnet/opus/haiku/fable)、run_in_background、name、isolation(worktree/remote) |
| 2 | **Bash** | **command**、timeout、description、run_in_background、dangerouslyDisableSandbox |
| 3 | **TaskOutput** | **task_id**、block、timeout |
| 4 | **ExitPlanMode** | allowedPrompts（deprecated） |
| 5 | **Edit(FileEdit)** | **file_path**、**old_string**、**new_string**、replace_all |
| 6 | **Read(FileRead)** | **file_path**、offset、limit、pages(PDF) |
| 7 | **Write(FileWrite)** | **file_path**、**content** |
| 8 | **Glob** | **pattern**、path |
| 9 | **Grep** | **pattern**、path、glob、output_mode(content/files_with_matches/count)、-B、-A、-C、context、-n、-i、-o、type、head_limit、offset、multiline |
| 10 | **TaskStop** | task_id、shell_id(deprecated) |
| 11 | **ListMcpResources** | server |
| 12 | **RefreshMcpTools** | server |
| 13 | **Mcp** | [k:string]: unknown |
| 14 | **NotebookEdit** | **notebook_path**、cell_id、new_source、cell_type、edit_mode |
| 15 | **ReadMcpResourceDir** | **server**、**uri** |
| 16 | **ReadMcpResource** | **server**、**uri** |
| 17 | **ReportFindings** | **findings**、level（ultrareview 专用） |
| 18 | **TodoWrite** | **todos[{content, status, activeForm}]** |
| 19 | **WebFetch** | **url**、**prompt** |
| 20 | **WebSearch** | **query**、allowed_domains、blocked_domains |
| 21 | **AskUserQuestion** | **questions[1-4]{question, header, options[2-4]{label, description, preview}}** |
| 22 | **SendFeedback** | **type**、**title**、**details**、area |
| 23 | **ClaudeDesign** | **operation**、**arguments** |
| 24 | **Projects** | **method**(info/read/search/write/delete)、path、content、local_path、present_to_user、query、n |
| 25 | **EnterPlanMode** | （空） |
| 26 | **TaskCreate** | **subject**、**description**、activeForm、metadata |
| 27 | **TaskGet** | **taskId** |
| 28 | **TaskUpdate** | **taskId**、subject、description、activeForm、status(+deleted)、addBlocks、addBlockedBy、owner、metadata |
| 29 | **TaskList** | （空） |
| 30 | **REPL** | **code**、description、timeout |
| 31 | **Workflow** | script、name、args、scriptPath、resumeFromRunId |
| 32 | **CronCreate** | **cron**、**prompt**、recurring、durable |
| 33 | **CronDelete** | **id** |
| 34 | **CronList** | （空） |
| 35 | **ScheduleWakeup** | delaySeconds、reason、prompt、stop |
| 36 | **RemoteTrigger** | **action**(list/get/create/update/run)、trigger_id、body |
| 37 | **ShowOnboardingRolePicker** | （空） |
| 38 | **Monitor** | **description**、deadline |
| 39 | **ProposeSkills** | **proposals[1-3]{name, kind(new/improvement), target, description, evidence, skillMd}** |
| 40 | **Artifact** | action(publish/list)、file_path、favicon、limit、scope、title、description、label、url、force |
| 41 | **PushNotification** | **message**、status |
| 42 | **EnterWorktree** | name、path |
| 43 | **ExitWorktree** | **action**(keep/remove) |

### 2. 二进制内额外发现的工具（SDK 类型未暴露，但运行时存在）

`read_clipboard`、`write_clipboard`、`wait` 等（约 3+ 个，带 grant 权限门控：`clipboardRead`/`clipboardWrite`）。

> **结论：Claude Code 2.1.218 官方工具面约 43 个，加内部工具约 46+ 个。**

---

## 二、anyCode 当前内置工具清单（`crates/tools/src/registry.rs`）

FileRead、FileWrite、Bash、Glob、Grep、Edit、NotebookEdit、TodoWrite、PlanWrite、WebFetch、WebSearch、
Mcp、ListMcpResources、ReadMcpResource、McpAuth、Lsp、
Agent、SkillSearch、Skill、SendMessage、LegacyTaskAgent、
TaskCreate、TaskUpdate、TaskList、TaskGet、TaskStop、TaskOutput、
TeamCreate、TeamDelete、CronCreate、CronUpdate、CronDelete、CronList、RemoteTrigger、
EnterPlanMode、ExitPlanMode、EnterWorktree、ExitWorktree、ToolSearch、Sleep、StructuredOutput、
PowerShell、Config、SendUserMessage、Brief、AskUserQuestion、Repl、
SpeechToText、TextToSpeech、GenerateImage、GenerateVideo、KnowledgeSearch、
Browser 组（BrowserNavigate/Tabs/Snapshot/Click/Type/PressKey/Scroll/Screenshot/Cdp）、
微信/知识库等扩展工具。

**anyCode 核心工具约 53 个 + 浏览器 9 个 + 扩展工具。**

---

## 三、逐项对比：参数是否一致？

### 3.1 完全一致的（9 个）

| 工具 | 结论 |
|------|------|
| Edit | file_path/old_string/new_string/replace_all 完全一致 |
| Write | file_path/content 一致 |
| Glob | pattern/path 一致 |
| NotebookEdit | notebook_path/cell_id/new_source/cell_type/edit_mode 一致 |
| ReadMcpResource | server/uri 一致 |
| ListMcpResources | server 一致 |
| ExitPlanMode | 一致（CC 的 allowedPrompts 已 deprecated，anyCode 无亦可） |
| EnterPlanMode | 一致（空入参） |
| TaskList | 一致（空入参） |

### 3.2 参数不一致，需升级（14 个）

| 工具 | Claude Code 参数 | anyCode 现状 | 升级项 |
|------|------------------|--------------|--------|
| **Bash** | command, timeout, description, run_in_background, dangerouslyDisableSandbox | command, timeout_ms, run_in_background | ① 参数名 `timeout_ms` → `timeout`（对齐）；② 增加 `description`；③ 增加 `dangerouslyDisableSandbox` |
| **Read** | file_path, offset, limit, pages | 仅 file_path | 增加 `offset`、`limit`（行范围读取）、`pages`（PDF 分页） |
| **Grep** | pattern, path, glob, output_mode, -B/-A/-C/context, -n, -i, -o, type, head_limit, offset, multiline | 仅 pattern, path | 大项：增加 output_mode / glob / 上下文 / head_limit / offset / multiline / -i / -o / type |
| **TodoWrite** | todos[{content, status, activeForm}] | todos[{id, content, status}] | ① 移除 `id`（CC 无）；② 增加 `activeForm` |
| **WebFetch** | url, prompt（均必填） | url 必填、prompt 可选 | prompt 改为必填语义 |
| **WebSearch** | query, allowed_domains, blocked_domains | 仅 query | 增加 allowed_domains / blocked_domains |
| **Agent** | description, prompt, subagent_type, model, run_in_background, name, isolation(worktree/remote) | prompt/task, description, agent_type, subagent_type, cwd, model, isolation, run_in_background | ① 增加 `name`（可寻址）；② isolation 增加 `remote`；③ `cwd` 保留为 anyCode 扩展 |
| **AskUserQuestion** | questions[1-4]{question, header, options[2-4]{label, description, preview}} | question/header/options/multiSelect | ① 增加 `preview` 字段；② 约束 1-4 问、2-4 选项 |
| **TaskCreate** | subject, description, activeForm, metadata | subject, description, metadata | 增加 `activeForm` |
| **TaskGet** | taskId | id | 参数名 `id` → `taskId` |
| **TaskUpdate** | taskId, subject, description, activeForm, status, addBlocks, addBlockedBy, owner, metadata | id, subject, description, status, metadata | ① `id` → `taskId`；② 增加 activeForm/addBlocks/addBlockedBy/owner |
| **TaskStop** | task_id, shell_id | id | 参数名对齐为 `task_id`（兼容 shell_id） |
| **CronCreate** | cron, prompt, recurring, durable | schedule, command, schedule_timezone, session_id, failure_destination, tool_profile | ① 参数名 `schedule`→`cron`、`command`→`prompt`（或双兼容）；② 增加 recurring/durable；③ anyCode 独有字段保留 |
| **ExitWorktree** | action(keep/remove) | 无参 | 增加 `action` |

### 3.2.1 P0 实施状态（2026-08-01 已落地，`cargo check -p anycode-tools` + 全量测试通过）

| 工具 | 状态 | 落地说明 |
|------|------|----------|
| **Bash** | ✅ 已对齐 | schema 增加 `timeout`（`timeout_ms` 保留为弃用别名）、`description`、`dangerouslyDisableSandbox`；`effective_timeout = timeout.unwrap_or(timeout_ms)`；`dangerouslyDisableSandbox` 时跳过沙箱 cwd 解析；description 透传给后台任务标题 |
| **Read** | ✅ 已对齐 | 增加 `offset`（1-based）/`limit` 行范围流式读取；`pages` 参数明确返回不支持（PDF 分页）；返回 `offset/limit/end_line/total_lines_seen/truncated` |
| **Grep** | ✅ 已对齐 | 增加 `glob`、`output_mode`（content/files_with_matches/count）、`-B/-A/-C/context`、`-n`、`-i`、`-o`、`type`、`head_limit`、`offset`、`multiline`；content 模式 `rg --json` 分页 |
| **TodoWrite** | ✅ 已对齐 | 移除必填 `id`（自动生成 Uuid），增加 `activeForm`（present continuous form 描述） |
| **TaskCreate** | ✅ 已对齐 | 增加 `activeForm` |
| **TaskGet/TaskUpdate/TaskStop/TaskOutput** | ✅ 已对齐 | 参数名 `id` → `taskId`（TaskGet/TaskUpdate）/`task_id`（TaskStop/TaskOutput），旧值兼容；TaskUpdate 增加 activeForm/owner/addBlocks/addBlockedBy |
| **CronCreate** | ✅ 已对齐 | `schedule`/`command` 双兼容 `cron`/`prompt`（二者任一可用），增加 `recurring`/`durable`；anyCode 独有字段保留 |
| **WebSearch** | ✅ 已对齐 | 增加 `allowed_domains`/`blocked_domains`，custom 端点请求体透传 |
| **WebFetch** | ⏳ 未对齐 | prompt 仍是可选（anyCode 语义更宽松，可接受） |
| **Agent** | ⏳ 未对齐 | 缺 `name`；isolation 未含 `remote`（`cwd` 为 anyCode 扩展，保留） |
| **AskUserQuestion** | ⏳ 未对齐 | 缺 `preview` 字段与 1-4/2-4 数量约束 |
| **ExitWorktree** | ⏳ 未对齐 | 缺 `action`（keep/remove） |

### 3.3 anyCode 有、Claude Code 没有的（14 个，保留为差异化）

PlanWrite、McpAuth、Lsp、SkillSearch、Skill、SendMessage、LegacyTaskAgent、TeamCreate、TeamDelete、CronUpdate、ToolSearch、Sleep、StructuredOutput、KnowledgeSearch、SpeechToText/TextToSpeech/GenerateImage/GenerateVideo、浏览器全套、微信工具。

### 3.4 Claude Code 有、anyCode 没有的（11 个，建议新增）

| 工具 | 用途 | 建议 |
|------|------|------|
| **Workflow** | 自包含 workflow 脚本（agent()/parallel()/phase()） | anyCode 已有 `crates/tools/src/workflows/` 骨架，建议对齐为正式工具 |
| **RefreshMcpTools** | 刷新 MCP 工具列表 | 简单，建议新增 |
| **ReadMcpResourceDir** | 列出 MCP 目录资源 | 简单，建议新增 |
| **SendFeedback** | 反馈提交 | 可选 |
| **ReportFindings** | 代码审查结果上报 | 与 anyCode 评审流程整合时新增 |
| **ClaudeDesign** | 设计项目操作 | 与 canvas-design skill 整合 |
| **Projects** | 云端项目文档 CRUD | 与 anyCode 云端 account-service 整合 |
| **ScheduleWakeup** | 动态循环唤醒 | 建议新增（与 Cron 互补） |
| **Monitor** | 后台监控任务 | 建议新增（anyCode 有 daemon/scheduler 基础） |
| **ProposeSkills** | 技能提案 | 建议新增（anyCode 已有技能系统） |
| **Artifact / PushNotification** | 制品发布 / 推送 | 可选（与云端控制台整合） |

---

## 四、升级方案（按优先级）

### P0 — 参数对齐（✅ 2026-08-01 全部落地，`cargo check -p anycode-tools` + 157 单测 + 9 集成测试通过）

1. **Grep 扩展**：增加 output_mode、glob、context(-B/-A/-C)、head_limit、offset、multiline、-i、-o、type。✅
2. **Read 扩展**：offset/limit（大文件分段读取）；pages（PDF 分页，明确返回不支持）。✅
3. **Bash 对齐**：`timeout_ms` → `timeout`（兼容两者），加 `description`、`dangerouslyDisableSandbox`。✅
4. **Task 系列参数名对齐**：TaskGet/TaskUpdate 的 `id` → `taskId`；TaskStop/TaskOutput `id` → `task_id`（旧值兼容）；TaskCreate/TaskUpdate 增加 activeForm/owner/addBlocks/addBlockedBy。✅
5. **TodoWrite**：移除 id、增加 activeForm。✅
6. **CronCreate**：`schedule`/`command` 双兼容 `cron`/`prompt`，增加 recurring/durable。✅
7. **WebSearch**：增加 allowed_domains/blocked_domains，custom 端点请求体透传。✅

### P1 — 新增工具（✅ 2026-08-01 全部落地）

7. **Workflow** 正式工具 ✅ → 实现为 `WorkflowGet`（`crates/tools/src/workflows/`），支持内联 `script` / `scriptPath` / 工作目录自动发现（workflow.yml、workflow.yaml、.anycode/workflow.*），返回解析后的步骤、DAG 拓扑层、校验问题（`PlanValidationResult`）与 checkpoint 进度；执行仍由 scheduler（channel-bridge）负责，工具只读。
8. **RefreshMcpTools**、**ReadMcpResourceDir** ✅ → `crates/tools/src/mcp_tools.rs` 注册，含 MCP 会话刷新与目录资源列举。
9. **Monitor**（后台监控，利用 scheduler 基础）✅ → `crates/tools/src/orchestration.rs` 注册。
10. **ScheduleWakeup**（动态循环）✅ → `crates/tools/src/orchestration.rs` 注册（与 Cron 互补）。
11. **ProposeSkills**（技能提案）✅ → `crates/tools/src/agent_tools.rs` 实现并注册：校验 1-3 个技能提案（name/kind/target/description/evidence/skillMd），返回结构化评审（名称冲突、target 存在性、SKILL.md frontmatter 可解析性）；只读不落盘。

> 回归：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets`、`cargo test -p anycode-core`（50）、`cargo test -p anycode-tools`（165 单测 + 9 集成）全部通过；新增工具均已加入 `DEFAULT_TOOL_IDS` 与 governance 目录（`TOOL_CATALOG`）。

### P2 — 差异化保留（不动）

- anyCode 独有工具（PlanWrite、LSP、微信、浏览器、媒体生成、KnowledgeSearch、Team/CronUpdate 等）继续保留，这是 anyCode 相比 CC 的差异化优势。

---

## 五、总结

- **Claude Code 2.1.218：约 43 个官方工具 + 内部工具 3+，共 46+。**
- **anyCode：约 62+ 工具（含浏览器与扩展）。**
- **工具名层面：高度对齐，anyCode 基本覆盖 CC 核心工具；参数层面：14 个工具参数不一致（Grep/Read/Bash/Task 系列最突出），11 个 CC 工具 anyCode 缺失。**
- **建议：先做 P0 参数对齐（影响最大、改动集中在 schema + 入参解析），再做 P1 新增工具；P2 差异化保留。**