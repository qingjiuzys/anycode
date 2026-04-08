# anyCode

Terminal-first AI assistant: ask questions and run tasks from the CLI, or bridge the same runtime to WeChat, Telegram, or Discord.

**Languages:** [简体中文](README.zh.md)

- **Docs site:** <https://qingjiuzys.github.io/anycode/>
- **CLI binary:** `anycode`
- **License:** [MIT](LICENSE)

## Quick start (3 steps)

1. Install anyCode
2. Run `anycode setup` to configure the model and optional channels
3. Run a task to verify

**macOS / Linux:**

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

**Windows PowerShell:**

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

**After install:**

```bash
anycode --help
anycode setup
anycode run --agent general-purpose "Reply with OK only"
```

If you see `command not found`, check PATH notes in the install guide.

## Documentation

- [Getting started](docs-site/guide/getting-started.md)
- [Install](docs-site/guide/install.md)
- [Troubleshooting](docs-site/guide/troubleshooting.md)
- [Full docs directory](docs-site/guide/docs-directory.md)

**Chinese (仓库内 Markdown):** [快速开始](docs-site/zh/guide/getting-started.md) · [安装](docs-site/zh/guide/install.md) · [排错](docs-site/zh/guide/troubleshooting.md) · [文档地图](docs-site/zh/guide/docs-directory.md)

## For developers

```bash
cargo fmt
cargo clippy
cargo test --workspace
```

Preview the docs site locally:

```bash
cd docs-site && npm install && npm run dev
```
