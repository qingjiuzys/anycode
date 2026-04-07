model-banner = == anyCode 模型 / 鉴权 ==
model-config-path = 配置文件：
model-main-menu-title = 主菜单
model-menu-global = 配置全局默认 LLM（provider / model / key）
model-menu-routing = 配置按任务路由 routing.agents（不同 agent 不同模型/厂商）
model-menu-exit = 退出
model-pick-prompt = 请选择
model-invalid = 输入无效。
model-menu-fallback-1 =   1) 配置全局默认 LLM
model-menu-fallback-2 =   2) 配置按任务路由 routing.agents
model-menu-fallback-0 =   0) 退出
model-pick-number = 请选择编号：
wizard-pick-model-prompt = 选择模型
wizard-pick-anthropic-prompt = 选择 Claude 模型
wizard-prompt-model-id = 模型 id（如 gpt-4o / 网关内模型名）
wizard-bedrock-endpoint-prompt = Bedrock 自定义 endpoint（可选；回车用 AWS 默认）
wizard-copilot-model-prompt = Copilot 模型 id（须含 claude，如 claude-sonnet-4）
wizard-model-id-non-tty = 请输入 model id（回车使用默认）：
wizard-api-key-prompt = API Key（必填；直接回车保留已有）
wizard-base-url-prompt = Base URL（回车使用推荐默认）
wizard-base-url-merge-pty = Base URL（回车使用推荐默认；可清空为官方默认）
wizard-base-url-merge-fallback = Base URL（回车为 {$url}；仅输入空格可尝试清空）：
wizard-saved = 已写入 {$path}
wizard-no-config = 未检测到配置文件：{$path}
wizard-no-config-model = 未检测到 {$path}，请先运行 anycode model 或 anycode config
wizard-run-config-first = 先运行：anycode config
wizard-model-empty = model 不能为空。
wizard-unknown-model = 未知 model：{$id}。可用模型：{$list}
wizard-provider-not-supported = 当前 provider={$p} 不支持 anycode model set（请用 anthropic/claude 或 z.ai）
wizard-model-set-ok = 已设置默认模型为：{$model}
model-provider-title = 模型 / 鉴权提供方（Model/auth provider）
model-pick-provider = 请选择提供方
model-provider-list = 提供方列表：
model-back-menu = Back（返回主菜单）
model-zai-auth-title = Z.AI 鉴权 / 端点（Z.AI auth method）
model-back = Back
model-current-global = 当前全局：provider={$p} plan={$l} model={$m}
model-placeholder-hint = {$label}：{$hint}
model-custom-agent = 自定义 agent_type（手动输入）
model-pick-agent-type = 选择要覆盖的 agent_type
model-enter-number = 编号：
model-edit-routing = 编辑 routing.agents["{$key}"]（空字符串=清除该字段或删除 profile）
model-keep-global = （回车保留/跳过；provider 空=沿用全局 {$p}）
model-prompt-provider = provider
model-routing-title = 按任务路由（routing.agents）
model-placeholder-default-hint = 请改用 Custom Provider 或已支持网关
model-catalog-placeholder = {$label} — 占位（未接入）{$hint}
model-routing-updated = ✅ 已更新 routing.agents → {$path}
model-prompt-provider-fallback = provider（空=全局 {$p}）：
model-prompt-model-skip = model（空=跳过）：
model-prompt-plan-skip = plan（空=跳过）：
model-prompt-api-key-profile = 专用 api_key
model-prompt-api-key-skip = api_key（空=跳过）：
model-prompt-base-url-skip = base_url（空=跳过）：
cfg-wizard-title = == anyCode 配置向导 ==
cfg-wizard-v1 = V1 仅支持：z.ai（= BigModel）
cfg-wizard-path = 配置文件：~/.anycode/config.json
cfg-existing-hint = 检测到已有配置：直接回车可保留默认值。
cfg-plan-step-pty = Step 1/4：选择套餐（上下选择，回车确认）
cfg-plan-coding = 编码套餐（推荐，Coding endpoint）
cfg-plan-general = 通用套餐（通用 endpoint）
cfg-plan-step-fallback-title = Step 1/4：选择套餐
cfg-plan-invalid = 输入无效，请输入 1 或 2。
cfg-model-step-pty = Step 2/4：选择模型（上下选择，回车确认）
cfg-model-glm5 = glm-5（推荐）
cfg-model-glm47 = glm-4.7（兼容）
cfg-model-custom = 自定义（手动输入）
cfg-model-step-fallback-title = Step 2/4：选择模型
cfg-model-invalid = 输入无效，请输入 1-3。
cfg-model-custom-pty = 请输入自定义 model（如 glm-5）
cfg-model-custom-fallback = 请输入自定义 model（如 glm-5，回车默认 glm-5）：
cfg-api-step-pty = Step 3/4：请输入 API Key（必填，输入会隐藏）
cfg-api-step-fallback = Step 3/4：请输入 API Key（必填）：
cfg-api-empty = API Key 不能为空。
cfg-base-step-title = Step 4/4：Base URL（可选）
cfg-base-prompt-pty = 输入 Base URL（可选，回车使用默认）
cfg-base-prompt-fallback = 输入 Base URL（可选，回车默认 {$url}）：
cfg-saved = ✅ 配置已保存到 ~/.anycode/config.json
cfg-next-example-title = 下一步示例：
cfg-next-example-cmd =   anycode run --agent general-purpose "用中文帮我分析这个项目"
cfg-wechat-hint-non-tty = 提示：绑定微信并安装登录自启后台桥可运行：anycode channel wechat
cfg-wechat-confirm = 是否现在绑定微信并安装登录自启后台桥？
cfg-skip-wechat = 已跳过微信绑定（--skip-wechat）。
cfg-no-config-warn = ⚠️  Warning: 未检测到 {$path}
cfg-no-config-run =    先运行：anycode config
cfg-accent-base-url = Base URL（可选）
zai-model-custom = 自定义（手动输入）
anthropic-model-custom = 自定义（手动输入）
zai-model-catalog-entry = {$api}（{$display}）
anthropic-model-catalog-entry = {$id}（{$title}）
err-model-required = model 不能为空。
err-permission-mode = 无效的 security.permission_mode：{$mode}。允许值：default, auto, plan, accept_edits, bypass
err-provider = 无效的 provider：{$p}。请使用 anycode model 列出目录中的厂商 id，或查阅文档
err-unknown-zai-model = 未知 model：{$id}。可用模型：{$list}
err-no-home-memory = 无法解析主目录（HOME 未设置），无法解析 memory.path 默认路径
err-memory-backend = 无效的 memory.backend：{$b}（允许 noop、none、off、file、hybrid）
err-config-not-found = 配置文件不存在：{$path}
err-read-system-prompt = 读取 system 提示文件 {$path} 失败
log-ignore-approval-session = 本进程跳过工具交互审批（-I / --ignore-approval / ANYCODE_IGNORE_APPROVAL；配置文件未改写）
log-wechat-bridge-no-approval = 微信桥：已关闭工具交互审批（该进程无 TUI 确认渠道；配置中的 require_approval 未写回磁盘）
err-anthropic-api-key = Anthropic 为全局 provider 时 api_key 不能为空
err-anthropic-routing-key = 路由中使用 Anthropic 时，请在 config.json 的 provider_credentials 中设置 "anthropic" 密钥，或在对应 routing profile 上设置 api_key
err-github-copilot-token = GitHub Copilot 为全局 provider 时请在 api_key 填入 GitHub PAT，或先运行 `anycode model auth copilot`
err-github-copilot-routing-key = 路由中使用 GitHub Copilot 时，请设置 provider_credentials[\"github_copilot\"]、profile api_key，或 ~/.anycode/credentials/github-oauth.json
log-mcp-json-skip = ANYCODE_MCP_SERVERS JSON 解析失败，已忽略: {$err}
log-mcp-json-array = ANYCODE_MCP_SERVERS 应为 JSON 数组
log-mcp-entry-skip = ANYCODE_MCP_SERVERS[{$i}] 缺少 command 或非 HTTP 的 url，已跳过
log-ignore-deny-pattern = 忽略无效 security.mcp_tool_deny_patterns 条目 {$pat}: {$err}
log-memory-backend-internal = 内部错误：未知 memory.backend {$b}（应为 noop|file|hybrid）
err-bootstrap-orch = ToolServices 编排状态加载失败: {$err}
log-mcp-stdio-ok = MCP stdio 已连接 slug={$slug}
log-mcp-stdio-fail = MCP stdio 连接失败 slug={$slug} err={$err}
log-mcp-http-ok = MCP Streamable HTTP 已连接 slug={$slug} url={$url}
log-mcp-http-fail = MCP HTTP 连接失败 slug={$slug} url={$url} err={$err}
log-llm-session = 🧠 LLM 会话: openai_compat_stack={$openai} anthropic={$anthropic} bedrock={$bedrock} github_copilot={$copilot}
log-memory-info = 🗃 memory backend={$backend} path={$path} auto_save={$auto}
err-memory-file-store = FileMemoryStore: {$err}
err-memory-hybrid-store = HybridMemoryStore: {$err}
workspace-readme = # anyCode 工作区\n\n本目录为 anyCode 用户级默认工作区根（类似 OpenClaw 的 `~/.openclaw/workspace`）。\n\n- **`projects/index.json`**：你从各项目目录运行 `anycode` 时登记的目录列表（按最近使用时间排序）。\n- **记忆（Memory）**：仍在 `~/.anycode/config.json` 的 `memory.path` 配置，与本目录并列，未自动合并至此。\n- **微信桥**：新绑定账号时默认将 `workingDirectory` 设为本目录规范路径；可在微信内用 `/cwd` 改为项目路径。\n\n详见 `docs/cli.md`。
