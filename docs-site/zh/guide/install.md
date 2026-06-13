---
title: 安装
description: 通过 GitHub Release、install.sh 或 Cargo 安装 anyCode。
summary: 先给普通用户最短安装路径，再给进阶安装方式。
read_when:
  - 在新环境安装 anyCode。
  - 在预编译与源码构建之间做选择。
---

# 安装

适合想“尽快可用”、不希望先研究源码构建的用户。

完成本页后，你会得到：

- `anycode` 可执行
- `anycode --help` 验证通过
- 安装失败时的替代路径

## Recommended path

| OS | Recommended install |
|----|---------------------|
| **macOS** | 从 [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) 下载 **`anyCode_<version>_aarch64.dmg`** — 内置 CLI + 工作台 + 原生 STT/OCR。macOS 不再单独发布 CLI tar.gz。 |
| **Linux** | `curl ... install.sh \| bash` |
| **Windows** | `irm ... install.ps1 \| iex` |

### macOS 桌面应用（推荐）

1. 打开 [Releases](https://github.com/qingjiuzys/anycode/releases)，下载 **`anyCode_<version>_aarch64.dmg`**。
2. 打开 DMG，将 **anyCode** 拖入「应用程序」。
3. 启动 **anyCode** — 自动拉起内置 CLI sidecar（`anycode dashboard`）并打开工作台。

CLI 位于 App 包内（macOS Release 不再单独附 CLI 包）：

```bash
/Applications/anyCode.app/Contents/Resources/resources/bin/anycode --help
```

仅在无 GUI 或需要 PATH 独立 CLI 时，macOS 才使用 `install.sh`。

## 一行安装（Linux）

本仓库为 **`qingjiuzys/anycode`**：

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

预期输出：安装脚本下载二进制并默认执行 `anycode setup`（交互终端上包含记忆 / 向量步骤；见 [记忆系统说明](./memory)）。

## 一行安装（Windows PowerShell）

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

预期输出：PowerShell 安装完成，默认进入 setup（除非禁用）。

或：

```bash
export ANYCODE_GITHUB_REPO="qingjiuzys/anycode"
bash scripts/install.sh
```

预期输出：安装脚本按环境变量指定仓库执行。

常用选项：`--version latest` 或 `--version v0.2.2`（固定 release tag）；`--bin-dir "$HOME/.local/bin"`；`--dry-run`；`--no-setup`（跳过安装后向导）；`--quiet`（减少下载输出）；`--method auto`（允许回退源码安装）。安装成功后默认会执行 `anycode setup`，且在交互终端默认显示下载进度。完整说明：

```bash
curl -fsSL "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --help
```

下一步：确认参数后，重新执行安装命令。

## 安装固定版本

默认一行命令安装 [GitHub Releases 最新版](https://github.com/qingjiuzys/anycode/releases/latest)。若要固定当前工作区版本：

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode --version v0.2.2
```

预期输出：安装固定版本 `v0.2.2`。

或（Cargo 直接按 tag 安装）：

```bash
cargo install --git https://github.com/qingjiuzys/anycode --tag v0.2.2 anycode --force
```

Release 页面：<https://github.com/qingjiuzys/anycode/releases>

**各平台 Release 资产：**

| 平台 | GitHub Release 资产 |
|------|---------------------|
| macOS（Apple Silicon） | `anyCode_<version>_aarch64.dmg`（CLI 已内置） |
| Linux x86_64 / arm64 | `anycode-<target>.tar.gz` |
| Windows x86_64 / arm64 | `anycode-<target>.zip` |

本地构建 macOS 桌面：`./scripts/build-desktop-release.sh`。

## 安装后验证

```bash
anycode --help
anycode setup
```

预期输出：`--help` 显示命令列表；`setup` 打开向导。

如果提示 `command not found`，按安装脚本输出里的 PATH 提示处理后，开一个新终端再试。

## 从源码安装（进阶）

给贡献者或需要自定义构建的用户：

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
# 直接运行：./target/release/anycode --help
```

预期输出：release 构建成功，产物位于 `target/release`。

安装到 PATH（推荐，避免 `command not found`）：

```bash
./scripts/install.sh --source-dir "$(pwd)"
anycode --help
```

下一步：执行 `anycode setup`。

## 仅本地克隆

```bash
./scripts/install.sh --source-dir "$(pwd)" --bin-dir "$HOME/.local/bin"
```

## 可选功能

例如启用 MCP（`tools-mcp`）：

```bash
cargo build -p anycode --features tools-mcp
```

预期输出：构建出的二进制包含 MCP 能力。

## 下一步

- [快速开始](./getting-started) — `setup` 与首条任务  
- [配置与安全](./config-security) — `~/.anycode/config.json`  
