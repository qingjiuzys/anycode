

# anyCode

<p align="center">
  <strong>企业级 · 开源 (MIT) · 自托管</strong>
</p>

<p align="center">
  本地优先 · BYOK · 数据保留在本地<br/>
  在您的硬件上运行代理，而非黑盒云网关。
</p>

<p align="center">
  <a href="https://gitee.com/nuai/anycode/blob/master/README.zh.md">中文文档</a> ·
  <a href="https://anycode.work/docs/">官方文档</a> ·
  <a href="https://github.com/qingjiuzys/anycode/releases">下载页面</a> ·
  <a href="LICENSE">MIT 许可证</a>
</p>

<p align="center">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="rust" src="https://img.shields.io/badge/rust-2021-orange.svg" />
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg" />
  <img alt="status" src="https://img.shields.io/badge/status-invite--only%20beta-yellow.svg" />
</p>

---

## 简介

anyCode 是一款企业级 AI 编程助手，采用本地优先架构，代码和数据默认保存在您的本地环境中。与传统云端 AI 编程工具不同，anyCode 让您完全掌控数据和模型选择。

### 核心特性

| 特性 | 说明 |
|------|------|
| **本地执行** | 代码和文件默认保存在本地，不上传云端 |
| **BYOK 模型** | 支持 30+ 模型提供商，自由选择或自托管模型 |
| **自托管部署** | 完全本地化部署，满足合规要求 |
| **团队协作** | 支持 LAN 发现和云端 A2A 流式交接 |
| **企业功能** | 审批流程、API 令牌、项目策略等 |
| **定时任务** | 自然语言配置的 cron 调度器 |
| **Skills 扩展** | 可安装的技能包，支持文档、表格、PPT 等 |

### 技术架构

```
┌─────────────────────────────────────────────────────────┐
│  anyCode.app / 浏览器                                    │
│  Digital Workbench · Grill/Goal · 协作界面              │
└───────────────────────────┬─────────────────────────────┘
                            │ loopback :43180
┌───────────────────────────▼─────────────────────────────┐
│  AgentRuntime (Rust / Tokio)                            │
│  LLM 提供商 · 工具 · Skills · 审批 · 内存管理            │
└───────────────┬─────────────────────────┬───────────────┘
                │                         │
        ~/.anycode/                 anycode-daemon
        config · sessions           调度器 (cron)
                │
                ▼ 可选云服务
        anycode.work — 设备链接 · 组织成员 · 流式交接
```

---

## v0.3.0 新功能

### 拷问模式（Grill Me）

启用 Grill 模式（`/grill-me` 或 `/拷问`），代理先对齐思路，再开始实现：

- 通过 `AskUserQuestion` 每次只问一个问题
- 每个问题包含推荐选项
- 可在代码库中回答的问题会优先从代码库获取答案
- 只有在您确认后（如 "go ahead" / 「可以动手了」）才开始实现

避免"在确认目标之前重写整个代码库"这类失败模式。

### 多人协作交接（Team Handoff）

将项目或会话移交给同事，而非压缩包或截图：

- **局域网**：mDNS 发现 + 点对点传输
- **云端团队**：同组织设备间通过 A2A 流式中继
- **显式同意**：接收方在 Desktop 中批准后才发放流令牌
- Portal 团队页面列出组织成员和在线的 Desktop 实例

---

## 快速开始

### macOS（推荐）

从 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 `anyCode_<version>_aarch64.dmg`，将 anyCode 拖入应用程序文件夹。

### Linux / Windows（无头模式）

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

安装完成后，打开 `http://127.0.0.1:43180/setup` 进行配置。

**验证**：发送"回复 OK 即可"测试。

---

## 详细功能

### Digital Workbench

项目管理、会话、资产、自动化和安全审批 —— 全部在本地 Workbench 中进行。macOS 用户使用 **anyCode.app**，开发构建通过 `http://127.0.0.1:43180` 访问。

### 单一 Rust 代理运行时

`AgentRuntime` 统一管理多轮 LLM + 工具循环，支持 Bash、Edit、Grep、MCP、LSP、Skills、Cron、知识库等工具。

### BYOK 模型目录

支持 30+ 提供商（z.ai/GLM、DeepSeek、Anthropic、Bedrock、Copilot、OpenRouter、Ollama、自定义 OpenAI 兼容端点等）。密钥保存在 `~/.anycode/config.json`。

### 拷问与目标模式

- **拷问模式**：苏格拉底式的计划对齐
- **目标模式**：自主迭代直至满足门控条件

### 团队交接

发现同伴 → 请求交接 → 同伴批准 → 流式传输 `handoff_v1` 包。语义对齐 [Google A2A](https://google.github.io/A2A/) 概念，同时保持"数据留本地"的姿态。

### 内置调度器

通过 `anycode-daemon scheduler` 配置自然语言 cron 任务，包含运行历史、防护机制和 Workbench 会话输出。

### Skills 与办公交付

可安装的 Skills（包括文档/表格/演示文稿/PDF 流水线）—— 代理不仅可以生成代码，还能产出办公文档。

### macOS 原生能力

**Apple Speech**（无需下载 Whisper）和 **Apple Vision OCR** 集成到 Desktop shell 中。

### 企业扩展接口

本地 REST API、API 令牌、项目策略、权限模式、评估/门控工具。SSO/RBAC 正在规划中。

---

## 开发指南

```bash
# 代码检查
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace

# Desktop 迭代
./scripts/sync-desktop-dev.sh          # 仅 UI
./scripts/sync-desktop-dev.sh --rust   # UI + Rust (release-local)
```

本地预览用户文档：

```bash
cd crates/account-portal && npm install && npm run dev
# → http://127.0.0.1:43201/docs
```

**技术栈**：Rust workspace · Tokio · React · Tauri · Fluent i18n

---

## 文档资源

| 主题 | 链接 |
|------|------|
| 入门指南 | [https://anycode.work/docs/guide/getting-started](https://anycode.work/docs/guide/getting-started) |
| 安装部署 | [https://anycode.work/docs/guide/install](https://anycode.work/docs/guide/install) |
| Desktop (macOS) | [https://anycode.work/docs/guide/desktop](https://anycode.work/docs/guide/desktop) |
| 模型配置 | [https://anycode.work/docs/guide/models](https://anycode.work/docs/guide/models) |
| Workbench | [https://anycode.work/docs/guide/workbench](https://anycode.work/docs/guide/workbench) |
| 定时任务 | [https://anycode.work/docs/guide/cli-scheduler](https://anycode.work/docs/guide/cli-scheduler) |
| 故障排查 | [https://anycode.work/docs/guide/troubleshooting](https://anycode.work/docs/guide/troubleshooting) |

---

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

---

<p align="center">
  <sub>您掌控的企业级代理 —— 本地执行、显式交接、动手前先对齐。</sub>
</p>