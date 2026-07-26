# 审计存疑项 — 待用户决策（2026-07-24，07-26 更新）

> 用户已授权「按照你的倾向来」。以下记录各问题的处理结果；仅剩 Q5.3/Q7.5/Q7.6/Q8 余项待后续。

## 已按倾向执行完毕

- **Q1 A**：config schema 忽略未知键（channels/wechatHistory 本就不在 schema）；statusLine/terminal 保留但标记 DEPRECATED。
- **Q2 A**：FDE 提为全管线默认，lingqi 降级 `templates/lingqi/` + 可选品牌。
- **Q4 A**：已提交 4 个 commit（渠道移除 / experience+office+FDE / fmt / provider 加固）。
- **Q5 A**：dead-code 全清（channel-bridge 12 死依赖、bootstrap dialoguer、desktop BridgeState、dashboard 6 处、agent 5 处）。
- **Q6 A**：workflow DAG 已接入 scheduler（cron job `workflow` 字段 → DAG+checkpoint；文档已更新 docs/user/zh/guide/daemon.md）。
- **Q7.1/7.2/7.4**：/setup/* 写端点仅 loopback 免认证；desktop 杀端口进程前校验 anycode 身份；3 个死环境变量已删。
- **Q9.1**：配额耗尽 429 快速失败（zai+openai）；anthropic 过载重试加强（10 次/30s/抖动）。
- **Q9.3**：eval 超时会话自动 cancel。

## Kimi 验收实测（07-26）

- 端到端 **PROVEN**：dashboard → Kimi(kimi-k2-turbo-preview) → 流式中文回答 → session completed。
- 沿途抓到并修复 3 个真实 provider bug：① content block 缺 `type` 标签（严格网关 400）；② 尾部 system 角色消息（违反 Anthropic 契约）；③ 错误日志无响应体。
- 全套件验收受阻：61 工具/12k token 大请求持续触发 Kimi 套餐 rate_limit（engine overloaded），与 Claude Code 本会话共享预算；属环境约束非代码缺陷。anthropic 重试已加强到 10 次/30s 仍不够覆盖长窗口，引擎冷却后可重跑：`ANYCODE_CHAT_PROVIDER=anthropic ANYCODE_CHAT_MODEL=kimi-k2-turbo-preview ANYCODE_CHAT_BASE_URL=https://api.kimi.com/coding/v1/messages ANYCODE_CHAT_API_KEY_ENV=KIMI_API_KEY python3 scripts/run-agent-quality.py --models kimi-k2-turbo-preview --arms experience_skill`

## 仍待决策（下轮）

- **Q7.3** WebChatHub stub 移除（API 面清理）。
- **Q7.5** sync_registry 启动本地模型即改写全局 chat 配置——改 per-agent 路由还是保持？
- **Q7.6** stale-epoch 分支：防御性代码还是掩盖竞态？
- **Q8 余项** weak-local 恢复双实现保留（语义真实不同：消息锁/预算/返回类型各异），已注释说明。
- **Q3** hidden split 评测何时跑（需引擎空闲窗口）。










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
