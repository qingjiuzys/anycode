# anyCode

面向终端用户的 AI 助手。装好后可以在命令行里提问、执行任务，也可以接入微信 / Telegram / Discord 走同一套能力。

**语言:** [English README](README.md)

- 在线文档: [https://qingjiuzys.github.io/anycode/](https://qingjiuzys.github.io/anycode/)
- 可执行命令: `anycode`
- 许可: [MIT](LICENSE)

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
anycode run --agent general-purpose "Reply with OK only"
```

如果提示 `command not found`，先看安装文档中的 PATH 说明。

## 文档入口

- [快速开始](docs-site/zh/guide/getting-started.md)
- [安装](docs-site/zh/guide/install.md)
- [排错](docs-site/zh/guide/troubleshooting.md)
- [文档地图](docs-site/zh/guide/docs-directory.md)

**English (repo Markdown):** [Getting started](docs-site/guide/getting-started.md) · [Install](docs-site/guide/install.md) · [Troubleshooting](docs-site/guide/troubleshooting.md) · [Docs directory](docs-site/guide/docs-directory.md)

## 给开发者

**实现技术栈：** Rust workspace（`cargo`）；异步运行时 **Tokio**；终端 UI **ratatui** + **crossterm**；Markdown **pulldown-cmark**；国际化 **Fluent**（`fluent-bundle`）；代码高亮 **syntect**。逻辑拆在 `anycode-core`、`anycode-agent`、`anycode-llm`、`anycode-tools`（MCP/LSP）等 crate。

TTY 下 **`anycode repl` 流式界面**默认 **备用屏全屏**（与 `terminal_guard::stream_repl_use_alternate_screen` 一致）；若需旧版主缓冲 Inline + 宿主 scrollback，可设 **`ANYCODE_TERM_REPL_INLINE_LEGACY=1`**。维护者说明见仓库内 [`docs/stream-repl-layout.md`](docs/stream-repl-layout.md)。

```bash
cargo fmt
cargo clippy
cargo test --workspace
```

文档本地预览:

```bash
cd docs-site && npm install && npm run dev
```

