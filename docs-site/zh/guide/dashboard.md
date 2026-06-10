---
title: 打开数字工作台
description: 启动本地工作台或 macOS 桌面应用。
---

# 打开数字工作台

## 浏览器方式（通用）

```bash
anycode dashboard --open
```

浏览器会打开 `http://127.0.0.1:43180`。若已登录本地账户，可直接使用。

第一次打开若是空页面，先在某个项目目录用终端跑一次任务，再回到工作台即可看到数据。

## macOS 桌面应用

1. 在 [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) 下载对应平台安装包（Apple Silicon / Intel 的 `.dmg`，Windows 的 `.msi` 或 `.exe`，Linux 的 `.deb` 或 `.AppImage`）。
2. 打开 DMG，将 **anyCode** 拖入「应用程序」。
3. 从启动台打开应用；会自动启动内置工作台。

本地自行打包：`./scripts/build-desktop-release.sh`

## 接下来看什么

- [工作台导览](./workbench) — 每个侧栏页面做什么
- [终端里怎么用](./cli) — 在终端里和助手协作
- [定时提醒](./cli-scheduler) — 创建定时任务

English: [Open the Workbench](/guide/dashboard).
