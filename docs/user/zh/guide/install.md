---
title: 安装
description: 安装 anyCode 桌面应用或无头守护进程 — 企业级开源免费 Agent，支持独立部署。
---

# 安装

> **企业级开源免费 Agent** — 支持 **独立部署**（本机、内网服务器或无头 `anycode-daemon`）。无需把业务数据交给第三方云端。

![anyCode 独立部署 — 本机工作台](/docs/assets/screenshots/home.png)
*图：安装后在本机运行的 Digital Workbench*

## 推荐方式（macOS）

1. 打开 [GitHub Releases](https://github.com/qingjiuzys/anycode/releases)
2. 下载 **`anyCode_<version>_aarch64.dmg`**（Apple Silicon）或对应 Intel 包
3. 打开 DMG，将 **anyCode** 拖入「应用程序」
4. 从启动台打开 **anyCode** — 内置工作台会自动启动

无需单独安装 CLI。日常在应用窗口内使用即可。

## Linux / Windows

| 平台 | 方式 |
|------|------|
| **Linux 桌面** | Release 页的 `.deb` / `.AppImage`（如有） |
| **Linux 服务器** | 安装 **`anycode-daemon`**，浏览器访问 Workbench |
| **Windows** | Release 页的 `.msi` / `.exe`（如有） |

一行安装脚本（Linux）：

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode
```

## 安装后检查

1. 打开 anyCode 或访问 `http://127.0.0.1:43180`
2. 若跳转到 **设置向导**（`/setup`），按提示完成模型配置
3. 发一条测试消息确认对话正常

## 从源码构建（开发者）

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
./scripts/sync-desktop-dev.sh --rust   # 本地迭代
# 或正式包：./scripts/build-desktop-local.sh
```

## 下一步

- [快速开始](./getting-started)
- [打开工作台](./dashboard)
- [常见问题](./troubleshooting)

English: [Install](/docs/guide/install).
