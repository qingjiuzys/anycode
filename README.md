# anyCode

**Rust 终端 AI 编程助手**：全屏 TUI、行式 REPL、多轮工具调用、可选 HTTP Daemon。默认面向本机开发；一条 **`anycode setup`** 可完成工作区、API 向导与可选 **微信扫码绑定**，与手机共用同一套 Agent。

**在线文档（GitHub Pages）：** https://qingjiuzys.github.io/anycode/

CLI 二进制名：**`anycode`**  
协议：**MIT** — 见仓库根目录 [`LICENSE`](LICENSE)

---

## Documentation（说明文档）

完整、中英对称的说明文档在 **`docs-site/`**（VitePress）。在仓库内预览：

```bash
cd docs-site && npm install && npm run dev
```

构建（与 CI 一致）：

```bash
cd docs-site && npm install && npm run build
```

**入口**

| | |
|--|--|
| English | [Docs directory](docs-site/guide/docs-directory.md) · [Getting started](docs-site/guide/getting-started.md) |
| 简体中文 | [文档地图](docs-site/zh/guide/docs-directory.md) · [快速开始](docs-site/zh/guide/getting-started.md) |
| 站点维护约定 | [docs-site/README.md](docs-site/README.md) |

**在线文档（GitHub Pages）**：推送 `main`/`master` 且变更 `docs-site/**` 或工作流文件时，Actions **Docs** 会构建并发布。仓库 **Settings → Pages** 中 **Source** 选 **GitHub Actions** 后，项目站地址一般为：

**https://qingjiuzys.github.io/anycode/**

（与仓库 `qingjiuzys/anycode` 同名路径；本地 `npm run dev` 仍用根路径 `/`，生产构建由 CI 设置 `VITEPRESS_BASE=/anycode/`。）

---

## 一行安装（可选）

默认指向本仓库 **`qingjiuzys/anycode`**；详见 [安装文档](docs-site/zh/guide/install.md)。

macOS / Linux:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

Release 附件命名、源码构建、可选 **`tools-mcp`**、Daemon、路由、MVP/路线图等：**以文档站为准**，不再在 README 重复长文。

---

## 参与贡献

```bash
cargo fmt
cargo clippy
cargo test --workspace
```

修改默认暴露给模型的工具时，遵守 **`crates/tools/src/registry.rs`** 顶部 checklist 与 **`SECURITY_SENSITIVE_TOOL_IDS`**。详见 [开发与贡献](docs-site/zh/guide/development.md)。

---

## 许可证

本项目以 **[MIT License](LICENSE)** 开源。
