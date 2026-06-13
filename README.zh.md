# anyCode

面向终端、**自托管 BYOK** 的 AI 助手：在命令行提问与执行任务，也可把同一套 runtime 桥接到**个人微信**、Telegram 或 Discord，或通过本地 **Digital Workbench** 管理项目、会话与定时任务。

**语言:** [English README](README.md)

- 在线文档: [https://qingjiuzys.github.io/anycode/](https://qingjiuzys.github.io/anycode/)
- 可执行命令: `anycode`
- 许可: [MIT](LICENSE)

## 与其他工具的不同之处

- **单一 Rust runtime** — 一个 `AgentRuntime` 编排 LLM + 工具（Bash、Edit、Grep、MCP、LSP、Skills、Cron、Knowledge 等）。Agent 在本机执行，不是云端托管 Gateway。
- **个人微信桥** — iLink 扫码绑定；手机下发任务、微信内审批敏感工具、回传文件/图片。见 [微信与 setup](docs-site/zh/guide/wechat.md)。
- **本地 Digital Workbench** — `anycode dashboard --open` 查看项目、会话、资产、自动化、安全审批，并提供 REST API 供二次集成。见 [工作台导览](docs-site/zh/guide/workbench.md)。
- **自动化** — 自然语言 cron、运行历史、项目 guardrails，可选微信通知。见 [定时任务](docs-site/zh/guide/cli-scheduler.md)。
- **BYOK 模型目录** — 与 OpenClaw 对齐的 30+ provider（z.ai/GLM、DeepSeek、Anthropic、Bedrock、Copilot、OpenRouter、Ollama、自定义端点等）。见 [模型与端点](docs-site/zh/guide/models.md)。
- **企业二次开发更友好** — 本地 Workbench REST API、API Token、项目策略、eval/门禁 harness、权限模式文档化。SSO/RBAC 在路线图中，尚未生产就绪。
- **macOS 体验更好** — **anyCode.app** Tauri 壳内置 Workbench，并提供 **Apple Speech**（原生语音识别，无需下载 Whisper）与 **Apple Vision OCR**（设备端提取文字）。仅浏览器访问 `127.0.0.1:43180` 时无法使用这些原生能力。

## 模型验证范围

anyCode 集成了多家 LLM，但**维护者日常验证**主要集中在：

- **z.ai / 智谱 GLM**（默认对话栈，如 `glm-5`）
- **DeepSeek**（OpenAI 兼容 API，含 tool schema 规范化）

**CI** 使用**本地 Mock OpenAI 兼容服务**覆盖 agent loop，**不调用**真实厂商 API。

目录中其余 provider 均为**可配置支持**。配置凭据后，请用 `anycode status`、工作台模型探测或一次短对话自测。详见 [模型与端点](docs-site/zh/guide/models.md)。

## 3 步上手

1. 安装 anyCode
2. 运行 `anycode setup` 完成模型与 channel 配置
3. 运行一次任务验证

macOS / Linux:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

安装后验证:

```bash
anycode --help
anycode setup
anycode run --agent general-purpose "请只回复：OK"
```

如果提示 `command not found`，先看安装文档中的 PATH 说明。

**打开工作台（可选）：**

```bash
anycode dashboard --open
```

**macOS 桌面应用：** 从 [Releases](https://github.com/qingjiuzys/anycode/releases) 安装，获得原生 STT/OCR 与 sidecar 打包体验。

## 文档入口

- [快速开始](docs-site/zh/guide/getting-started.md)
- [安装](docs-site/zh/guide/install.md)
- [模型与端点](docs-site/zh/guide/models.md)
- [工作台导览](docs-site/zh/guide/workbench.md)
- [微信与 setup](docs-site/zh/guide/wechat.md)
- [定时任务](docs-site/zh/guide/cli-scheduler.md)
- [排错](docs-site/zh/guide/troubleshooting.md)
- [文档地图](docs-site/zh/guide/docs-directory.md)

**English (repo Markdown):** [Getting started](docs-site/guide/getting-started.md) · [Install](docs-site/guide/install.md) · [Models](docs-site/guide/models.md) · [Workbench](docs-site/guide/workbench.md) · [WeChat](docs-site/guide/wechat.md) · [Troubleshooting](docs-site/guide/troubleshooting.md) · [Docs directory](docs-site/guide/docs-directory.md)

## 给开发者

**实现技术栈：** Rust workspace（`cargo`）；异步运行时 **Tokio**；终端 UI **ratatui** + **crossterm**；Markdown **pulldown-cmark**；国际化 **Fluent**（`fluent-bundle`）；代码高亮 **syntect**。逻辑拆在 `anycode-core`、`anycode-agent`、`anycode-llm`、`anycode-tools`（MCP/LSP）等 crate。

TTY 下 **`anycode repl` 流式界面**默认 **备用屏全屏**（与 `terminal_guard::stream_repl_use_alternate_screen` 一致）；若需旧版主缓冲 Inline + 宿主 scrollback，可设 **`ANYCODE_TERM_REPL_INLINE_LEGACY=1`**。维护者说明见仓库内 [`docs/ops/stream-repl-layout.md`](docs/ops/stream-repl-layout.md)。

```bash
cargo fmt
cargo clippy
cargo test --workspace
cargo build --release -p anycode
```

文档本地预览:

```bash
cd docs-site && npm install && npm run dev
```
