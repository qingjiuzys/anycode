# anyCode × Claude Code 提示词差异明细

> 分析日期：2026-08-01
> 对比基准：Claude Code 2.1.218（`~/.local/share/claude/versions/2.1.218` 二进制字符串特征 + npm sdk-tools.d.ts） vs anyCode（`crates/agent/prompts/` 全部文件 + `system_prompt.rs` / `prompt_assembler.rs` / `prompt_catalog.rs` / `reply_language.rs` / `runtime/mod.rs` / `model_instructions.rs`）
> 结论先行：anyCode 的提示词**分段架构与 Claude Code 同构**（多段合成、动态注入、运行时上下文段），差异集中在 **Claude 有而 anyCode 缺的 8 类段/机制** 与 **anyCode 独有而 Claude 无的 5 类能力**。

---

## 一、anyCode 当前提示词结构（代码证据）

### 静态段（`crates/agent/prompts/core/` + locale，`prompt_catalog.rs` 按需拼接）

| # | 段 id | 文件 | 内容 | 条件 |
|---|-------|------|------|------|
| 1 | reply_language | `locale/zh|en/reply_language.md` | 回复语言规则（中文/英文） | 有语言配置时 |
| 2 | tone | `core/tone.md` | 身份：`You are an AI coding agent. Ground answers in tool results — never invent tool output.` | 总是 |
| 3 | environment | `core/environment.md` | cwd / OS / date | 总是 |
| 4 | agent_loop | `core/agent_loop.md` | 工具调用时机 + Discoverable verification 5 步 | 总是（含 `{tools}` 占位） |
| 5 | user_clarification | `core/user_clarification.md` | 歧义时用 AskUserQuestion | 总是 |
| 6 | media_generation | `core/media_generation.md` | 媒体生成规则 | 有 GenerateImage/Video 工具时 |
| 7 | plan_progress | `core/plan_progress.md` | PlanWrite 层级树规则 | 有 PlanWrite 工具时 |
| 8 | browser | `core/browser.md` | BrowserSnapshot 优先 | 有 Browser* 工具时 |
| 9 | skills_section | 动态 | SKILL.md 发现注入 | 有技能时 |
| 10 | custom_agent | 动态 | `# Custom Agent Instructions` + agent.description | 总是 |

### 配置/文件段（`prompt_assembler.rs` 顺序）

| # | 段 id | 来源 | 说明 |
|---|-------|------|------|
| 11 | model_instructions_file | `model_instructions_content` | 显式配置的 AGENTS.md |
| 12 | config_append | `system_prompt_append` | config.json append |
| 13 | model_instructions_discovered | `discover_model_instructions` | 向上搜索 AGENTS.md/CLAUDE.md 等 |
| 14 | task_append | per-task append | 任务级追加 |
| 15 | profile_overlay | `agent.system_prompt_overlay` | 代理配置 overlay |
| 16 | plugin_overlays | `append_plugin_overlays` | 内置插件 overlay |

### 运行时上下文段（`runtime/mod.rs::build_context_sections`）

| # | 段 | 说明 |
|---|-----|------|
| 17 | Runtime Mode | `## Runtime Mode` |
| 18 | Slash Commands | `## Slash Commands` 内置命令清单 |
| 19 | workspace_section | 工作区提示 |
| 20 | channel_section | 通道提示（微信等） |
| 21 | workflow_section | workflow 提示 |
| 22 | goal_section | 目标模式提示 |
| 23 | Relevant Memories | 记忆注入 |
| 24 | prompt_fragments | 配置片段 |
| 25 | `<!-- SYSTEM_PROMPT_DYNAMIC_BOUNDARY -->` | 动态边界标记 |

### 每 turn 注入（`reply_language.rs`）

- **ephemeral reminder**：每条请求末尾追加 user 角色提醒（中文/英文），带 `ephemeral` 标记不入历史；可叠加 `host_intent_hint`。

### 压缩后注入（`compact/post_compact.rs`）

- 压缩后追加 `## Context from recent file reads (before compaction)` 文件摘录段（对齐 Claude `createPostCompactFileAttachments`）。

---

## 二、Claude Code 提示词特征（二进制证据）

| 特征 | 证据字符串 | 对应 anyCode 段 |
|------|-----------|----------------|
| 身份 | `You are Claude Code, Anthropic's official CLI for Claude.` | tone（但 anyCode 无产品名身份） |
| 记忆与持久化 | `## Memory and other forms of persistence`、`## Memory scope` | Relevant Memories（无静态说明段） |
| 输出格式 | `## Output Format`、`## Output format (complete — do NOT look this up)` | **缺** |
| Hooks 配置 | `## Hooks Configuration`、`| PreCompact | "manual"/"auto" |` | plugin overlay（无 hooks 段） |
| MCP 集成 | `## MCP Server Integration`、`## MCP Connector (Beta)`、`## MCP tools (specific)`、`## MCP Tool Conversion Helpers` | **缺**（有 MCP 工具实现） |
| 环境 | `## Environments` | environment |
| 验证 | `## Verification: <one-line what changed>`、`## Verification page — required` | Discoverable verification（格式不同） |
| 系统提示分层 | `## System Prompts`、`## System tier — becomes the skill` | 多段合成（无分层说明） |
| 工作流与场景 | `## Workflows and surfaces`、`## Workflow for optimizing existing code` | workflow_section |
| 编码规范 | `# Coding Standards` | **缺**（无编码规范静态段） |
| **Token 预算提醒** | `@internal Emit a <total_tokens>N tokens left</total_tokens> block in the system prompt, after each tool result, and after each regular user prompt`；4 模式（infinite/fixed/countdown/padded-countdown）；`CLAUDE_CODE_TOTAL_TOKENS_REMINDER*` | **缺** |
| **子代理提示词追加** | `--append-subagent-system-prompt <prompt>`、`appendSubagentSystemPrompt` | **缺**（nested_task 无提示词追加） |
| 子代理工作指导 | `Use multiple agents when: ...`、worker 示例、`Be independently implementable in an isolated git worktree` | nested_task 指导（弱） |
| 权限解释 | `permission_explainer`、`description: '...' // one-line, shown in permission dialog` | 审批（无 AI 解释） |
| **工具使用偏好** | `ALWAYS use ${bd} for search tasks. NEVER invoke grep or rg as a ${ri} command.`（搜索必须用 Grep 工具而非 bash） | **缺**（agent_loop 无此偏好） |
| **Git 操作规范** | `Run a git status command to see all untracked files. Never use the -uall flag`、commit 后顺序验证 | **缺** |
| 输出风格 | Output style（default/Explanatory/Learning） | **缺**（无 output style 概念） |
| 推理块 | thinking blocks 透传（`RedactedThinkingBlock`、`thinking_delta`） | 取决于 LLM provider（无提示词层面） |

---

## 三、差异清单（Claude 有而 anyCode 缺 → 候选优化项）

| # | 差异项 | 影响 | 实现成本 | 优先级建议 |
|---|--------|------|----------|-----------|
| D1 | **Token 预算提醒**（`<total_tokens>` 4 模式） | 长会话模型可自我节流，减少截断/超限 | 中（prompt 模板 + 每轮 token 统计注入） | **P0** |
| D2 | **工具使用偏好段**（搜索用 Grep 而非 bash grep/rg） | 避免模型滥用 bash 搜索，权限与性能双优 | 低（静态段） | **P0** |
| D3 | **Git 操作规范段**（git status 用法、-uall 禁令、commit 验证） | 提交链路稳定，防止大仓库内存问题 | 低（静态段） | **P1** |
| D4 | **MCP 集成说明段**（`## MCP Server Integration` 等） | 模型理解 MCP 工具来源与降级 | 低（静态段 + 条件注入） | **P1** |
| D5 | **输出格式规范段**（`## Output Format (complete)`） | 输出结构稳定（最终回复/工具轮/边界） | 中（静态段 + 语言化） | **P1** |
| D6 | **子代理提示词追加机制**（`appendSubagentSystemPrompt`） | 用户可对子代理注入规则，治理更强 | 中（配置 + nested_task 注入） | **P2** |
| D7 | **权限解释器**（AI 生成权限申请原因） | 降低误批/误拒 | 高（LLM 调用 + 审批 UI） | **P2** |
| D8 | **Hooks 配置说明段** | 与插件体系配套 | 低（静态段） | **P3** |

## 四、anyCode 独有而 Claude Code 无（保留优势，勿删）

| # | 能力 | 说明 |
|---|------|------|
| A1 | **Reply language 多语言体系** | zh/en 双语言提示词 + 每 turn ephemeral reminder（Claude 无） |
| A2 | **Runtime Mode / Slash Commands 清单段** | 模式与命令注入（Claude 在 CLI 层处理） |
| A3 | **SYSTEM_PROMPT_DYNAMIC_BOUNDARY** | 动态段边界标记（Claude 无） |
| A4 | **条件注入**（media/plan/browser 按工具集裁剪） | 减少无效 token |
| A5 | **压缩后文件摘录注入** | 摘要不丢关键文件内容（Claude 同款，anyCode 已实现） |

---

## 五、建议（按 ROI 排序）

1. **先做 D1 + D2**：token 提醒与工具偏好都是低风险静态/准静态改动，直接提升长会话与搜索行为质量，且可配单测验证。
2. **再做 D3 + D4**：git 规范与 MCP 说明同为静态段，成本低、稳定收益。
3. **D5 输出格式段**需要语言化设计（zh/en 两版），与现有 reply_language 体系衔接。
4. **D6/D7/D8** 涉及机制级改动（配置注入、审批 LLM 调用、hooks 事件流），建议作为独立里程碑。

> 本清单仅覆盖**提示词（system prompt）层面**；工具 schema 差异已在前序 `claude-code-tools-vs-anycode.md` 中处理完毕。

---

## 六、实施状态（2026-08-01 全部落地 ✅）

| # | 差异项 | 落地位置 | 说明 |
|---|--------|----------|------|
| D1 | Token 预算提醒 | `crates/agent/src/runtime/budget.rs::token_budget_context_section` + `execute_task.rs` / `execute_turn.rs` 注入 | 任务配置 `token_budget_total` 时注入 `<total_tokens>` 段，长会话自我节流 |
| D2 | 工具使用偏好段 | `crates/agent/prompts/core/tool_preferences.md` + `prompt_catalog.rs::default_stack_sections` | 静态段常驻注入 |
| D3 | Git 操作规范段 | `crates/agent/prompts/core/git_workflow.md` | 静态段常驻注入（措辞避开 "global" 哨兵词） |
| D4 | MCP 集成说明段 | `crates/agent/prompts/core/mcp_integration.md` | 有 `mcp__` / `ListMcpResourcesTool` 工具时条件注入 |
| D5 | 输出格式规范段 | `crates/agent/prompts/core/output_format.md` + `prompts/locale/{zh,en}/output_format.md` | 按活动语言注入，回退英文 core |
| D6 | 子代理提示词追加 | `crates/agent/src/runtime/nested_task.rs::SUBAGENT_SYSTEM_APPEND` | 子代理 `system_prompt_append` 默认注入聚焦/精简提示 |
| D7 | 权限解释器 | `crates/security/src/approval_presenter.rs` | 优先展示模型自述原因（description/reason/explanation），回退规则式中文意图说明（Bash/Write/Edit/Read/Grep/Web/Cron/Task），最后才回退参数 JSON |
| D8 | Hooks 配置说明段 | `crates/agent/prompts/core/hooks_configuration.md` | 静态段常驻注入 |

**验证**：`cargo test -p anycode-agent -p anycode-security` → 158 + 3 + 12 全部通过；`cargo check -p anycode-agent -p anycode-security` 无告警。
