<p align="center">
  <img src="brand/anycode-mark.svg" width="72" alt="anyCode" />
</p>

<h1 align="center">anyCode</h1>

<p align="center">
  <strong>企业级 · 开源 (MIT) · 自托管</strong>
</p>

<p align="center">
  本地优先 · BYOK · 数据不出端<br/>
  在你自己的机器上跑 Agent，而不是把代码交给云端黑盒。
</p>

<p align="center">
  <a href="https://anycode.work/docs/">文档</a> ·
  <a href="https://github.com/qingjiuzys/anycode/releases">下载</a> ·
  <a href="LICENSE">MIT</a>
</p>

<p align="center">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-blue.svg" />
  <img alt="rust" src="https://img.shields.io/badge/rust-2021-orange.svg" />
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg" />
</p>

---

anyCode 是本地优先的企业级 AI 编程工作台：项目、会话、工具、审批与 Skills 都在本机运行；模型 BYOK，支持 30+ 提供商与本地轻量套件（STT/TTS/OCR）。

## 界面一览

### 首页

<p align="center">
  <img src="docs/user/assets/screenshots/readme/home.png" alt="anyCode 首页 — 选项目、选模型、一句话开干" width="920" />
</p>

<p align="center"><sub>选项目与模型，一句话启动 Agent 会话</sub></p>

### 用量

<p align="center">
  <img src="docs/user/assets/screenshots/readme/usage.png" alt="LLM Token 用量分析" width="920" />
</p>

<p align="center"><sub>按项目、模型、时间维度追踪 Token 与估算成本</sub></p>

### 计划树

<p align="center">
  <img src="docs/user/assets/screenshots/readme/plan-tree.png" alt="审阅计划并开始执行" width="920" />
</p>

<p align="center"><sub>Agent 生成可执行计划树，审阅后一键 Build 分步落地</sub></p>

### 本地轻量套件

<p align="center">
  <img src="docs/user/assets/screenshots/readme/local-models.png" alt="模型库 — 本地 STT/TTS 等轻量模型" width="920" />
</p>

<p align="center"><sub>Whisper、Apple Speech、Piper 等本地模型，按能力启用、一键切换</sub></p>

### 汇报

<p align="center">
  <img src="docs/user/assets/screenshots/readme/report.png" alt="工作汇报 — 跨项目 AI 摘要" width="920" />
</p>

<p align="center"><sub>基于本地会话与事件，自动生成跨项目工作汇报</sub></p>

---

## 快速开始

**macOS**：从 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 DMG，拖入「应用程序」。

**Linux / Windows**：

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

打开 `http://127.0.0.1:43180/setup` 配置模型，发送「请只回复：OK」验证。

---

## 文档

| | |
|---|---|
| 快速开始 | [getting-started](https://anycode.work/docs/zh/guide/getting-started) |
| 安装 | [install](https://anycode.work/docs/zh/guide/install) |
| 模型 | [models](https://anycode.work/docs/zh/guide/models) |
| 工作台 | [workbench](https://anycode.work/docs/zh/guide/workbench) |
| 排错 | [troubleshooting](https://anycode.work/docs/zh/guide/troubleshooting) |

---

MIT License — 详见 [LICENSE](LICENSE)。

<p align="center">
  <sub>执行在本地，模型你自选，动手前先对齐。</sub>
</p>
