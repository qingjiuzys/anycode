<p align="center">
  <img src="brand/anycode-mark.svg" width="72" alt="anyCode" />
</p>

<h1 align="center">anyCode</h1>

<p align="center">
  <strong>企业自主可控 Agent</strong>
</p>

<p align="center">
  本地优先 · BYOK · 数据不出端<br/>
  在你自己的机器上跑 Agent，而不是把代码与业务交给云端黑盒。
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="https://anycode.work/docs/zh/">文档</a> ·
  <a href="https://github.com/qingjiuzys/anycode/releases">Releases</a> ·
  <a href="LICENSE">MIT</a>
</p>

<p align="center">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="rust" src="https://img.shields.io/badge/rust-edition%202021-orange.svg" />
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg" />
  <img alt="status" src="https://img.shields.io/badge/status-%E5%8F%97%E9%82%80%E5%86%85%E6%B5%8B-yellow.svg" />
</p>

---

## 为什么是 anyCode

大多数 AI 编程助手把推理与上下文放在厂商云端。anyCode 反过来：

| | 云端托管 Agent | **anyCode** |
|---|---|---|
| 执行位置 | 厂商服务器 | **你的本机 / 内网** |
| 模型 | 绑定套餐 | **BYOK**，自选厂商或私有端点 |
| 代码与附件 | 上传到云 | **默认不出端** |
| 团队协作 | 共享云 workspace | **显式交接**，对方同意后再传 |
| 可控性 | 黑盒策略 | **审批、策略、Skills、REST API** |

适合需要**自主可控、合规落地、二次开发**的企业与团队。

---

## What's new in v0.3.0

本版本聚焦**人机对齐**与**团队协作**：

### 拷问模式（Grill Me）

输入 `/拷问` 或开启拷问开关。Agent **先对齐、后动手**：

- 一次只问一个问题（`AskUserQuestion`）
- 每个问题给出推荐选项
- 能查代码库的不问人
- 说「可以动手了」才开始实现

避免「一上来就改仓库、改完才发现目标不对」。

### 多人协作交接（Team Handoff）

把项目或会话交给同事，而不是丢文件或截图：

- **局域网**：mDNS 发现同事，点对点传输（ADR 015）
- **云端团队**：同组织设备经 A2A 流式中继交接（ADR 016）——**不经 OSS**，包体仅内存管道转发
- **显式同意**：对方在 Desktop 批准后才发流令牌
- Portal「团队」页可看组织成员与在线实例

---

## 特性

### 本地 Digital Workbench

项目、会话、资产、自动化、安全审批，全部在本机工作台完成。  
**anyCode.app**（macOS）内嵌 Workbench；开发构建也可访问 `http://127.0.0.1:43180`。

### 单一 Rust Agent Runtime

一个 `AgentRuntime` 编排多轮 LLM + 工具循环——Bash、Edit、Grep、MCP、LSP、Skills、Cron、Knowledge 等。不是「壳一层调远程 Gateway」。

### BYOK 模型目录

30+ Provider（智谱 GLM、DeepSeek、Anthropic、Bedrock、Copilot、OpenRouter、Ollama、自定义 OpenAI 兼容端点…）。密钥留在 `~/.anycode/config.json`。

> 维护者日常验证：**z.ai / GLM**、**DeepSeek**。其余为配置支持；CI 用本地 Mock，不打真实厂商 API。

### 拷问 / 目标模式

- **拷问**：苏格拉底式对齐计划  
- **目标模式**：自主迭代直到门禁/目标达成  

### 团队交接（LAN + Cloud A2A）

发现同事 → 请求交接 → 对方批准 → 流式传输 `handoff_v1` 包。语义对齐 [Google A2A](https://google.github.io/A2A/)，数据路径仍守「不出端」姿态。

### 内置定时任务

自然语言 cron、`anycode-daemon scheduler`、运行历史与 guardrails，结果回到 Workbench 会话。

### Skills 与办公交付

可安装 Skills（含文档 / 表格 / 演示 / PDF 等办公链路），把 Agent 从「写代码」扩展到「交付物」。

### macOS 原生能力

**Apple Speech**（无需下载 Whisper）、**Apple Vision OCR**——仅 Desktop 壳可用；纯浏览器访问 loopback 不含这些能力。

### 企业二次开发

本地 REST API、API Token、项目策略、权限模式、eval/门禁 harness。SSO/RBAC 在路线图中。

---

## 3 步上手

1. 安装 **anyCode.app**（macOS）或 **`anycode-daemon`**（Linux / Windows 无头）
2. 打开 Workbench **`/setup`**，配置模型
3. 发一条测试消息；试试 `/拷问` 或「发现同事」交接

### macOS（推荐）

从 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 `anyCode_<version>_aarch64.dmg`，拖入「应用程序」。

### Linux / Windows（无头）

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

启动后打开 `http://127.0.0.1:43180/setup`。

**验证：** 发送「请只回复：OK」。

---

## 架构一瞥

```text
┌─────────────────────────────────────────────────────────┐
│  anyCode.app / Browser                                  │
│  Digital Workbench  ·  拷问 / 目标  ·  同事交接 UI       │
└───────────────────────────┬─────────────────────────────┘
                            │ loopback :43180
┌───────────────────────────▼─────────────────────────────┐
│  AgentRuntime（Rust / Tokio）                           │
│  LLM providers · Tools · Skills · Approvals · Memory    │
└───────────────┬─────────────────────────┬───────────────┘
                │                         │
        ~/.anycode/                 anycode-daemon
        config · sessions           scheduler（cron）
                │
                ▼ 可选云端（账号 / A2A 信令）
        anycode.work  — 设备关联 · 组织成员 · 流式交接中继
                        （包体不落 OSS / 不落库）
```

---

## 文档

| | |
|---|---|
| 快速开始 | [getting-started](https://anycode.work/docs/zh/guide/getting-started) |
| 安装 | [install](https://anycode.work/docs/zh/guide/install) |
| 桌面应用 | [desktop](https://anycode.work/docs/zh/guide/desktop) |
| 模型与端点 | [models](https://anycode.work/docs/zh/guide/models) |
| 工作台 | [workbench](https://anycode.work/docs/zh/guide/workbench) |
| 定时任务 | [cli-scheduler](https://anycode.work/docs/zh/guide/cli-scheduler) |
| 排错 | [troubleshooting](https://anycode.work/docs/zh/guide/troubleshooting) |

维护者文档：[`docs/`](docs/) · ADR · [`docs/roadmap.md`](docs/roadmap.md)

---

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace

# Desktop 日常迭代
./scripts/sync-desktop-dev.sh          # 仅 UI
./scripts/sync-desktop-dev.sh --rust   # UI + Rust（release-local）
```

本地预览用户文档：

```bash
cd crates/account-portal && npm install && npm run dev
# → http://127.0.0.1:43201/docs
```

技术栈：Rust workspace · Tokio · React（Workbench）· Tauri（Desktop）· Fluent i18n。

---

## 状态与许可

当前为**受邀内测**（算法备案审核中）。请勿理解为全面公开可用或监管已批复。

许可：[MIT](LICENSE)

---

<p align="center">
  <sub>企业自主可控 Agent — 执行在本地，协作可交接，动手前先对齐。</sub>
</p>
