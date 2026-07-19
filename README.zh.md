# anyCode

**自托管 BYOK** AI 助手，配套本地 **Digital Workbench**：在浏览器或 **anyCode.app**（macOS）中对话与执行任务，通过 **`anycode-daemon`** 桥接到**个人微信**、Telegram 或 Discord，并管理项目、会话与定时任务。

**语言:** [English README](README.md)

- 在线文档: [https://anycode.work/docs/](https://anycode.work/docs/)
- 无头二进制: `anycode-daemon`
- 许可: [MIT](LICENSE)

## 与其他工具的不同之处

- **单一 Rust runtime** — 一个 `AgentRuntime` 编排 LLM + 工具（Bash、Edit、Grep、MCP、LSP、Skills、Cron、Knowledge 等）。Agent 在本机执行，不是云端托管 Gateway。
- **个人微信桥** — iLink 扫码绑定；手机下发任务、微信内审批敏感工具、回传文件/图片。见 [微信与配置](https://anycode.work/docs/zh/guide/wechat)。
- **本地 Digital Workbench** — 内嵌于 **anyCode.app** 或开发构建的 `http://127.0.0.1:43180`，管理项目、会话、资产、自动化、安全审批，并提供 REST API。见 [工作台导览](https://anycode.work/docs/zh/guide/workbench)。
- **自动化** — 自然语言 cron、运行历史、项目 guardrails，可选微信通知。见 [定时任务](https://anycode.work/docs/zh/guide/cli-scheduler)。
- **BYOK 模型目录** — 与 OpenClaw 对齐的 30+ provider（z.ai/GLM、DeepSeek、Anthropic、Bedrock、Copilot、OpenRouter、Ollama、自定义端点等）。见 [模型与端点](https://anycode.work/docs/zh/guide/models)。
- **企业二次开发更友好** — 本地 Workbench REST API、API Token、项目策略、eval/门禁 harness、权限模式文档化。SSO/RBAC 在路线图中，尚未生产就绪。
- **macOS 体验更好** — **anyCode.app** Tauri 壳内置 Workbench，并提供 **Apple Speech**（原生语音识别，无需下载 Whisper）与 **Apple Vision OCR**（设备端提取文字）。仅浏览器访问 `127.0.0.1:43180` 时无法使用这些原生能力。

## 模型验证范围

anyCode 集成了多家 LLM，但**维护者日常验证**主要集中在：

- **z.ai / 智谱 GLM**（默认对话栈，如 `glm-5`）
- **DeepSeek**（OpenAI 兼容 API，含 tool schema 规范化）

**CI** 使用**本地 Mock OpenAI 兼容服务**覆盖 agent loop，**不调用**真实厂商 API。

目录中其余 provider 均为**可配置支持**。配置凭据后，请用工作台模型探测或一次短对话自测。详见 [模型与端点](https://anycode.work/docs/zh/guide/models)。

## 3 步上手

1. 安装 **anyCode.app**（macOS）或 **`anycode-daemon`**（Linux/Windows 无头）
2. 打开 Workbench **`/setup`** 配置模型与可选通道
3. 在工作台发送一条测试消息

**macOS（推荐）：** 从 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 **`anyCode_<version>_aarch64.dmg`**，拖入「应用程序」。桌面应用自动嵌入工作台。

**Linux / Windows（无头）：**

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

然后在启动桌面应用或开发版内嵌 dashboard 后访问 `http://127.0.0.1:43180/setup`。

**验证：** 在会话中发送「请只回复：OK」。

## 文档

用户文档发布在官网 **https://anycode.work/docs/**（源码：`docs/user/`）。

- [快速开始](https://anycode.work/docs/zh/guide/getting-started)
- [安装](https://anycode.work/docs/zh/guide/install)
- [桌面应用](https://anycode.work/docs/zh/guide/desktop)
- [无头守护进程](https://anycode.work/docs/zh/guide/daemon)
- [模型与端点](https://anycode.work/docs/zh/guide/models)
- [工作台导览](https://anycode.work/docs/zh/guide/workbench)
- [微信与配置](https://anycode.work/docs/zh/guide/wechat)
- [定时任务](https://anycode.work/docs/zh/guide/cli-scheduler)
- [排错](https://anycode.work/docs/zh/guide/troubleshooting)

## 开发者

**技术栈：** Rust workspace（`cargo`）；异步运行时 **Tokio**；Markdown **pulldown-cmark**；i18n **Fluent**。Runtime 分布在 `anycode-core`、`anycode-agent`、`anycode-llm`、`anycode-channel-bridge`、`anycode-tools` 等 crate。

```bash
cargo fmt
cargo clippy
cargo test --workspace
cargo build --release -p anycode-channel-bridge
cargo build --release -p anycode-desktop
```

本地预览文档（account-portal 开发服务器）：

```bash
cd crates/account-portal && npm install && npm run dev
```

打开 http://127.0.0.1:43201/docs
