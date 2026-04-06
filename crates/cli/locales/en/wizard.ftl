model-banner = == anyCode models / credentials ==
model-config-path = Config file:
model-main-menu-title = Main menu
model-menu-global = Configure global default LLM (provider / model / key)
model-menu-routing = Configure routing.agents (per agent type)
model-menu-exit = Exit
model-pick-prompt = Choose
model-invalid = Invalid input.
model-menu-fallback-1 = 1) Configure global default LLM
model-menu-fallback-2 = 2) Configure routing.agents
model-menu-fallback-0 = 0) Exit
model-pick-number = Enter number:
wizard-pick-model-prompt = Pick model
wizard-pick-anthropic-prompt = Pick Anthropic model
wizard-prompt-model-id = Model id (e.g. gpt-4o)
wizard-bedrock-endpoint-prompt = Bedrock endpoint override (optional; Enter for AWS default)
wizard-copilot-model-prompt = Copilot model id (must include "claude", e.g. claude-sonnet-4)
wizard-model-id-non-tty = model id (Enter for default):
wizard-api-key-prompt = API Key (required; empty keeps existing)
wizard-base-url-prompt = Base URL (Enter for default)
wizard-base-url-merge-pty = Base URL (Enter for recommended default; clear for official default)
wizard-base-url-merge-fallback = Base URL (Enter for {$url}; spaces only to try clear):
wizard-saved = Written to {$path}
wizard-no-config = No config file: {$path}
wizard-no-config-model = No config at {$path}. Run `anycode model` or `anycode config` first.
wizard-run-config-first = Run: anycode config
wizard-model-empty = model must not be empty.
wizard-unknown-model = Unknown model: {$id}. Available: {$list}
wizard-provider-not-supported = Provider {$p} does not support `anycode model set` (use anthropic/claude or z.ai)
wizard-model-set-ok = Default model set to: {$model}
model-provider-title = Model / auth provider
model-pick-provider = Choose provider
model-provider-list = Providers:
model-back-menu = Back (main menu)
model-zai-auth-title = Z.AI auth / endpoint
model-back = Back
model-current-global = Current global: provider={$p} plan={$l} model={$m}
model-placeholder-hint = {$label}: {$hint}
model-custom-agent = Custom agent_type (manual)
model-pick-agent-type = Choose agent_type to override
model-enter-number = Number:
model-edit-routing = Edit routing.agents["{$key}"] (empty = clear field or remove profile)
model-keep-global = (Enter to keep/skip; empty provider = use global {$p})
model-prompt-provider = provider
model-routing-title = Per-task routing (routing.agents)
model-placeholder-default-hint = Use Custom Provider or a supported gateway.
model-catalog-placeholder = {$label} — not wired yet{$hint}
model-routing-updated = Updated routing.agents → {$path}
model-prompt-provider-fallback = provider (empty = global {$p}):
model-prompt-model-skip = model (empty = skip):
model-prompt-plan-skip = plan (empty = skip):
model-prompt-api-key-profile = Profile api_key
model-prompt-api-key-skip = api_key (empty = skip):
model-prompt-base-url-skip = base_url (empty = skip):
cfg-wizard-title = == anyCode config wizard ==
cfg-wizard-v1 = V1: z.ai (= BigModel) only
cfg-wizard-path = Config: ~/.anycode/config.json
cfg-existing-hint = Existing config: press Enter to keep defaults.
cfg-plan-step-pty = Step 1/4: choose plan (↑↓, Enter)
cfg-plan-coding = Coding plan (recommended, Coding endpoint)
cfg-plan-general = General plan (general endpoint)
cfg-plan-step-fallback-title = Step 1/4: choose plan
cfg-plan-invalid = Invalid input; enter 1 or 2.
cfg-model-step-pty = Step 2/4: choose model (↑↓, Enter)
cfg-model-glm5 = glm-5 (recommended)
cfg-model-glm47 = glm-4.7 (compatible)
cfg-model-custom = Custom (manual)
cfg-model-step-fallback-title = Step 2/4: choose model
cfg-model-invalid = Invalid input; enter 1–3.
cfg-model-custom-pty = Custom model id (e.g. glm-5)
cfg-model-custom-fallback = Custom model (Enter for glm-5):
cfg-api-step-pty = Step 3/4: API Key (required, hidden)
cfg-api-step-fallback = Step 3/4: API Key (required):
cfg-api-empty = API Key cannot be empty.
cfg-base-step-title = Step 4/4: Base URL (optional)
cfg-base-prompt-pty = Base URL (optional, Enter for default)
cfg-base-prompt-fallback = Base URL (optional, default {$url}):
cfg-saved = Saved to ~/.anycode/config.json
cfg-next-example-title = Next:
cfg-next-example-cmd =   anycode run --agent general-purpose "your prompt"
cfg-wechat-hint-non-tty = To bind WeChat and install autostart bridge: anycode wechat
cfg-wechat-confirm = Bind WeChat and install login autostart bridge now?
cfg-skip-wechat = Skipped WeChat binding (--skip-wechat).
cfg-no-config-warn = ⚠️  Warning: no config at {$path}
cfg-no-config-run =    Run: anycode config
cfg-accent-base-url = Base URL (optional)
zai-model-custom = Custom (manual)
anthropic-model-custom = Custom (manual)
zai-model-catalog-entry = {$api} ({$display})
anthropic-model-catalog-entry = {$id} ({$title})
err-model-required = model must not be empty.
err-permission-mode = Invalid security.permission_mode: {$mode}. Allowed: default, auto, plan, accept_edits, bypass
err-provider = Invalid provider: {$p}. Run `anycode model` for ids or see docs.
err-unknown-zai-model = Unknown model: {$id}. Available: {$list}
err-no-home-memory = Cannot resolve home directory; cannot default memory.path
err-memory-backend = Invalid memory.backend: {$b} (allowed: noop, none, off, file, hybrid)
err-config-not-found = Config file not found: {$path}
err-read-system-prompt = Failed to read system prompt file {$path}
log-ignore-approval-session = Skipping interactive tool approval for this process (-I / --ignore-approval / ANYCODE_IGNORE_APPROVAL; config unchanged)
log-wechat-bridge-no-approval = WeChat bridge: tool approval disabled (no TUI; require_approval not written to disk)
err-anthropic-api-key = api_key is required when Anthropic is the global provider
err-anthropic-routing-key = For Anthropic in routing, set provider_credentials[\"anthropic\"] or profile api_key in config.json
err-github-copilot-token = For GitHub Copilot as global provider, set api_key (GitHub PAT) or run `anycode model auth copilot`
err-github-copilot-routing-key = For GitHub Copilot in routing, set provider_credentials[\"github_copilot\"], profile api_key, or ~/.anycode/credentials/github-oauth.json
log-mcp-json-skip = ANYCODE_MCP_SERVERS JSON parse failed, ignored: {$err}
log-mcp-json-array = ANYCODE_MCP_SERVERS must be a JSON array
log-mcp-entry-skip = ANYCODE_MCP_SERVERS[{$i}] missing command or non-HTTP url, skipped
log-ignore-deny-pattern = Skipping invalid security.mcp_tool_deny_patterns entry {$pat}: {$err}
log-memory-backend-internal = Internal error: unknown memory.backend {$b} (expected noop|file|hybrid)
err-bootstrap-orch = ToolServices orchestration state load failed: {$err}
log-mcp-stdio-ok = MCP stdio connected slug={$slug}
log-mcp-stdio-fail = MCP stdio connect failed slug={$slug} err={$err}
log-mcp-http-ok = MCP Streamable HTTP connected slug={$slug} url={$url}
log-mcp-http-fail = MCP HTTP connect failed slug={$slug} url={$url} err={$err}
log-llm-session = 🧠 LLM session: openai_compat_stack={$openai} anthropic={$anthropic} bedrock={$bedrock} github_copilot={$copilot}
log-memory-info = 🗃 memory backend={$backend} path={$path} auto_save={$auto}
err-memory-file-store = FileMemoryStore: {$err}
err-memory-hybrid-store = HybridMemoryStore: {$err}
workspace-readme = # anyCode workspace\n\nThis directory is the default anyCode user workspace root (similar to OpenClaw `~/.openclaw/workspace`).\n\n- **`projects/index.json`**: directories registered when you run `anycode` from a project (sorted by last use).\n- **Memory** paths stay in `~/.anycode/config.json` under `memory.path` (not merged here).\n- **WeChat bridge**: new binds default `workingDirectory` to this canonical path; use `/cwd` in WeChat to change.\n\nSee `docs/cli.md`.
