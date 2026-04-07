---
title: 安装
description: 通过 GitHub Release、install.sh 或 Cargo 安装 anyCode。
summary: 预编译包、一行安装脚本、cargo 安装与 Release 命名约定。
read_when:
  - 在新环境安装 anyCode。
  - 在预编译与源码构建之间做选择。
---

# 安装

## 方式怎么选

| 方式 | 适合 |
|------|------|
| **`scripts/install.sh`** | 想一条命令装好；优先下 Release，失败可回退 `cargo install --git` |
| **GitHub Releases 手动下载** | 只信浏览器 / 企业代理 |
| **`cargo install --git`** | 已装 Rust，跟分支或 tag |
| **本地 clone + `cargo build`** | 贡献者、改 feature |

## 一行安装

本仓库为 **`qingjiuzys/anycode`**：

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

或：

```bash
export ANYCODE_GITHUB_REPO="qingjiuzys/anycode"
bash scripts/install.sh
```

常用选项：`--version v0.1.0` 或 `latest`；`--bin-dir "$HOME/.local/bin"`；`--dry-run`；装完加 `--onboard`。完整说明：

```bash
curl -fsSL "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --help
```

## 安装 v0.1.0（固定版本）

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode --version v0.1.0
```

或（Cargo 直接按 tag 安装）：

```bash
cargo install --git https://github.com/qingjiuzys/anycode --tag v0.1.0 anycode --force
```

Release 页面：<https://github.com/qingjiuzys/anycode/releases/tag/v0.1.0>

## Release 附件命名

与脚本一致：`anycode-<rust-target>.tar.gz`，压缩包**根目录**含可执行文件 `anycode`。常见 target：`aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-unknown-linux-gnu`、`aarch64-unknown-linux-gnu`。

## 从源码

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
# ./target/release/anycode
```

安装到 PATH：

```bash
cargo install --path crates/cli --force
anycode --help
```

## 仅本地克隆

```bash
./scripts/install.sh --source-dir "$(pwd)" --bin-dir "$HOME/.local/bin"
```

## 可选功能

例如启用 MCP（`tools-mcp`）：

```bash
cargo build -p anycode --features tools-mcp
```

## 下一步

- [快速开始](./getting-started) — `onboard` 与首条任务  
- [配置与安全](./config-security) — `~/.anycode/config.json`  
