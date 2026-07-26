# 审计存疑项 — 待用户决策（2026-07-24）

> SOTA 迭代审计中无法自行拍板的问题。每条附背景、选项、我的倾向。
> 明显的问题已直接修复，不在此列（修复清单见文末）。

## 使用方式

在每条下面写「选 A/B/C」或自由回复即可。

---

## Q1. 用户 config.json 的残留键怎么处置

`~/.anycode/config.json` 里仍有已移除功能的配置：`channels`（telegram/discord/wechat 全 null）、`wechatHistory`、`terminal`、可能还有 `statusLine`。

- A. config schema 保留这些字段但标记 deprecated，启动时静默忽略（向后兼容，推荐）
- B. schema 删除字段 + 写一次性迁移把旧键从用户 config 里清掉
- C. 不动

**我的倾向：A**——不碰用户数据，schema 层忽略即可。

## Q2. lingqi 企业蓝风格的去留（已按 A 执行，知会即可）

已落地：FDE 模板提为 `templates/` 默认（lingqi 降级 `templates/lingqi/`）；所有 run 脚本默认 brand 改为 `fde-editorial`；scenarios 商用场景 manifest 默认 fde-editorial；experience pack 教学文案同步；`infer_brand_kit` 默认 fde-editorial。lingqi 保留为可选品牌。若你想让 lingqi 彻底移除，回复说明。

## Q3. M4 promotion gate 是否现在跑

`docs/ops/agent-quality-promotion.md` 的 hidden split 四臂评测 + 视觉盲评需要真实模型调用（deepseek-v4-flash / qwen3.8-max-preview 已在 config），会消耗 API 额度，时长可能 1-2 小时。

- A. 现在跑 dev split 冒烟（~10 任务），hidden 留给你决定
- B. 直接全量跑 hidden
- C. 本轮不跑，只保证 harness 可用

**我的倾向：A**。

## Q4. 282 个未提交文件的拆分方案

当前工作区混着两条线：① 渠道桥移除（已 staged 的删除 + 配套修改）② experience/评测/office 新体系（未跟踪）。加上本轮我的修复。

- A. 拆 3 个 commit：渠道移除 / experience+评测体系 / 本轮审计修复+风格对齐（推荐）
- B. 全部一个 commit
- C. 更细粒度（5+ 个 commit）

**我的倾向：A**。注：部分文件（task_compiler.rs、execute_task.rs 等）同时含②和③的改动，文件级无法干净三分；实际执行时建议 C1=渠道移除（39 staged 删除 + 渠道相关修改），C2=experience/评测/office+审计修复合并为一个，或接受近似拆分。**本轮我未执行任何 commit**——你确认方案后我再提交（或你说「直接提交」我就按 A 执行）。

## Q5. channel-bridge / dashboard 的 ~200 个 dead-code 警告

渠道移除后大量 unused import / dead fn 警告。全量清理会触碰很多文件。

- A. 本轮全清（推荐，配合渠道移除 commit）
- B. 只清 channel-bridge，dashboard 下轮
- C. 留着不管

**我的倾向：A**。

## Q6. Workflow DAG 执行链失去入口

ADR 014 决定「`depends_on` 以 DAG + checkpoint 执行」，但这条链（`tasks_run.rs` → `workflow_exec.rs` / `workflow_validate.rs`）的唯一调用方是被删除的终端 CLI `run` 命令。现在 rustc 报整链 dead code：scheduler 和 Workbench 都不走它。

- A. 把 workflow 执行接入 scheduler cron job（job 声明 workflow 文件 → DAG 执行），恢复 ADR 014 承诺的能力（推荐，工作量中等）
- B. 接入 Workbench 自动化面板（UI 更重）
- C. 删除这条链，ADR 014 第 6 条标记 superseded
- D. 暂时保留死代码，下轮再定

**我的倾向：A**。

## Q7. 安全与运维类存疑（需要你的判断）

1. **`api/auth.rs is_public_path` 把所有 `/api/setup/*`（含写端点）免认证**。dashboard 默认绑 127.0.0.1 风险低，但非 loopback 部署时可匿名改配置。建议：写端点仅限 loopback。要改吗？
2. **`dashboard_backend.rs` 启动时 `lsof -ti :43180 | kill`** 杀任意占用进程（不校验身份），可能误杀用户其他服务。建议：先校验进程是旧 anycode 再杀，或改成报错提示。要改吗？
3. **`WebChatHub` 已退化为恒 bail 的 stub**，但 AppState/handlers 仍带着它流转。建议下轮移除；本轮未动。
4. **三个只设不读的环境变量**（`ANYCODE_DASHBOARD_EMBEDDED_CHAT` / `INPROCESS_TRIGGERS` / `API_ONLY`）。删除还是恢复功能？
5. **`managed_local_llm.sync_registry` 启动本地模型就改写全局 provider/model/api_key**（api_key 写成 "sglang"/"ollama" 字面量），副作用大。这是有意设计（让会话立即切到本地模型）还是应改为 per-agent 路由？
6. **`chat_runtime/mod.rs` stale-epoch 早退分支疑似不可达**——防御性死代码还是掩盖真实竞态？需要你确认历史意图。

## Q8. execute_task / execute_turn 双编排器去重

审计发现 weak-local 工具恢复逻辑和 TaskCompiler 装配块在两个编排器里近乎逐行复制（execute_task.rs:317-386 ≈ execute_turn.rs:636-720；execute_task.rs:78-143 ≈ execute_turn.rs:90-248），且对同一 GuardDecision 的最终语义不一致（Partial vs 拼接近似成功文本）。合并是中等规模重构，本轮未动。

- A. 下轮迭代合并为单一编排实现（推荐）
- B. 保留双轨，先补文档说明语义差异

## Q9. 评测门禁校准（验收实测发现）

1. **qwen token-plan 周配额已耗尽**（07-30 05:38 UTC 重置）。第 2/3 轮验收后 5 个用例死于网关 429 → LLM 调用挂起。两个启示：a) 网关 429/配额错误应快速失败并明确报错，而不是挂起 40 分钟（建议下轮排查 streaming + failover 在 429 下的行为）；b) 验收套件需要支持切换备用模型（目前 dashboard server 只读 config 的 models.chat，无 env 覆盖；我想用隔离 HOME + deepseek 跑但被凭证复制安全限制拦下——需要一个官方的多模型验收通道）。
2. **trajectory gate `max_tool_errors=2` 偏紧**：eval 环境缺 browser bundle 时模型重试 BrowserNavigate 即超预算，任务其实已完成。建议区分"工具不可用（环境）"与"工具失败（agent）"计数。
3. **eval 会话泄漏**：超时用例的 session 永远停在 running。已修（executor 超时即调 cancel）。

---

## 已直接修复（无需回复）

6. **trajectory gate 误报（验收中抓获）**：`tool_result_injection.rs` 的 `preview_from_call` 把工具输入截断到 120 字符，`cd <workspace> && A/B` 全部被截成同一签名 → 评测轨迹门把 8/10 个真实完成的任务误判为"重复调用死循环"。预览上限提升到 2048。
7. **用例 bug**：`aq-web-dashboard-refactor` 断言期望 `index.html` 但 fixture/提示都是 `landing.html`（模型正确地原地改进了 landing.html）；`aq-web-form-responsive` 900s 超时不够（单 LLM 轮 ~280s），提到 1500s。
8. **账号服务 memory-sync**：push 无版本向量守卫，陈旧设备可覆盖新信封。加 `dominates()` 检查 + 单测。
9. **渠道死链大扫除**：channel-bridge 死依赖 12 个、locale 死键 ~150 行、i18n localize 死代码、bootstrap dialoguer-host feature 链、desktop BridgeState 空壳、dashboard 6 处死代码。
10. **评测 arm 失真**：`run-agent-quality.py` 原先把 ANYCODE_EVAL_* 只设在 runner 进程（dashboard server 读不到，四臂实为同一配置）。改为每臂起独立 dashboard server；共享生命周期代码抽到 `scripts/lib/dashboard_server.py`（visual/office 脚本去重）。
11. **验收实测**（qwen3.8-max-preview 真实模型，10 个真实场景）：landing 页 FDE 风格完全对齐参考页（serif 大标题/电蓝/mono 标签/hairline 网格）；工作汇报 docx 结构完整含 Decision/Action+责任人+日期；产品发布 deck 8 页含指标页。发现的唯一问题：模型仍用旧 lingqi 模板做 PPT → 已把 FDE 模板提为 `templates/` 默认、lingqi 降级 `templates/lingqi/`。

## 审计修复总表（第一轮，35+ 项）

- crypto：E2EE 主密钥弱熵（2^64→2^256）、解密 nonce panic、密钥文件权限竞争、nonce 自选构造改随机 96-bit、content_hash 改 keyed、device_id 漂移
- panic：router/skills UTF-8 truncate、`sk-` 密钥误判误杀正常记忆
- agent：deny 传播竞态、autosave 取错注入消息、infer_family 词边界、末轮 repair 错误原因、澄清问题不查 prompt、eval 双轨合一
- 验证器：first_path 提前返回、severity 三处硬编码、walkdir 无界+吞错、zip_openable 假检查
- 安装器：git 子路径回退必败、zip 根级 SKILL.md 必败、vet 静默通过、staging 残留
- dream：forget 全链路未接线、episode 不归档每晚重灌、save 吞错、max_promotions 逻辑错乱
- dashboard：recorder 双重入库、hydrate 截断方向错（留最旧丢最新）、CUDA 混入 mlx-lm
- bootstrap：跨 provider base_url 错继承（z.ai 网关打到 anthropic）
- office：PPTX 原生图表被 PNG 强制栅格化、build_content 背景色 bug、pdf rotate 失效、demo 优先级 bug、spreadsheet 假数据、eval prompt 旧风格
- 提示词：system.md 并入 tone.md，agent_loop.md 瘦身（ReAct 教学删除、TUI/REPL 引用改 Workbench）
- 死代码：plugins/prompt_assembler/GuardOutcome.termination/TaskCompiler.budgets/locale 字段/approval TUI 链等
