---
title: 快速开始
description: 安装 anyCode，完成工作台配置 — 企业级开源免费 Agent，支持在本机或内网独立部署。
---

# 快速开始

> **企业级开源免费 Agent** — 支持在你自己的机器或内网 **独立部署**。BYOK 模型，数据默认不出端。

![anyCode 企业级 Agent 工作台 — 支持独立部署](/docs/assets/screenshots/home.png)
*图：本地 Digital Workbench — 项目、会话、交付物与审批，全部在你的环境内运行*

面向**第一次使用 anyCode** 的用户。按下面四步走完，你就能在本机与 AI 助手对话。

## 你会得到什么

- anyCode 已安装（推荐 macOS 桌面应用）
- 已在工作台 **设置** 中配置好模型
- 一条测试对话成功返回

## 第一步：安装

macOS 用户从 [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) 下载 **`anyCode_<version>_aarch64.dmg`**，拖入「应用程序」后打开即可。

其他平台见 [安装](./install)。

## 第二步：打开工作台

启动 **anyCode** 后，应用内会自动打开数字工作台。

若使用无头服务（`anycode-daemon`），在浏览器访问 **`http://127.0.0.1:43180`**。

![工作台首页](/docs/assets/screenshots/home.png)
*图：工作台首页 — 从这里进入项目与会话*

## 第三步：完成首次配置

第一次使用会进入 **设置向导**（地址栏为 `/setup`）。按页面提示：

1. 选择模型服务商并填写 API Key（BYOK，密钥只保存在本机）
2. 按需开启记忆、向量检索等选项

![设置向导](/docs/assets/screenshots/setup.png)
*图：首次配置向导 — 选模型、填密钥*

完成后配置写入 **`~/.anycode/config.json`**。之后可随时在 **设置** 中修改。

![设置页](/docs/assets/screenshots/settings.png)
*图：设置 — 模型、通知、浏览器、Skills 等*

## 第四步：发一条测试消息

1. 在首页点击 **新建会话**，或进入某个 **项目**
2. 在输入框发送：

   > 请只回复：OK

3. 预期：助手回复 `OK`

若长时间无响应，见 [常见问题](./troubleshooting) 的「模型与网络」一节。

## 接下来做什么

| 你想… | 去看 |
|------|------|
| 了解侧栏每个页面 | [工作台导览](./workbench) |
| 让助手产出 PDF / 表格 / 幻灯片 | [会话交付物](./deliverables) |
| 每天定时跑任务 | [定时提醒](./cli-scheduler) |
| 换模型或改密钥 | [模型与端点](./models) |

## 文档在哪里

工作台侧栏底部有 **文档** 链接；也可直接打开 [anycode.work/docs](https://anycode.work/docs/)（本地开发时为 `http://127.0.0.1:43200/docs/`）。

![文档站点](/docs/assets/screenshots/docs-portal.png)
*图：在线文档 — 与产品同风格的说明页*

English: [Getting started](/docs/guide/getting-started).
