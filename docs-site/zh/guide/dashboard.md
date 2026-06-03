---
title: Digital Workbench（数字工作台）
description: 本地以项目为中心的会话、门禁、cron 与产出物看板。
---

# Digital Workbench（数字工作台）

**数字工作台**是 anyCode 的本地 Web 看板：按项目聚合 `run` / `goal` / `workflow` / `repl` / `cron` 会话、时间线事件、信任门禁、产出文件、Skills 与 cron 账本。数据写入 `~/.anycode/projects.db`，在使用 CLI 时自动更新。

## 快速开始

```bash
# 构建静态前端（UI 变更或发版前执行）
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh

# 推荐：release 内置 UI
cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

未构建 UI 时 release 仍可启动 API；`anycode dashboard doctor` 会提示缺少 UI。

## 桌面应用（macOS）

需要原生 Workbench 窗口（Tauri 壳 + 内置 `anycode dashboard` sidecar）时：

| 方式 | 说明 |
|------|------|
| **GitHub Release** | 在 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 `anyCode_<version>_aarch64.dmg`（Apple Silicon） |
| **本地构建** | `./scripts/build-desktop-release.sh` → `apps/anycode-desktop/target/release/bundle/dmg/` |

图标素材：[`anycode-logo.png`](https://qingjiuzys.github.io/anycode/anycode-logo.png)（源码在 `apps/anycode-desktop/assets/`）。详见 [`apps/anycode-desktop/README.md`](https://github.com/qingjiuzys/anycode/blob/main/apps/anycode-desktop/README.md)。

开发热更新：

```bash
anycode dashboard          # API 默认 :43180
cd crates/dashboard-ui && npm run dev   # Vite 将 /api 代理到上述服务
```

## 录制开关

默认会记录 `run`、`goal`、工作流步骤、流式 REPL 与 cron 调度。关闭：

```bash
export ANYCODE_DASHBOARD_RECORD=0
```

多实例时让 CLI 向指定服务推送 SSE：

```bash
export ANYCODE_DASHBOARD_URL=http://127.0.0.1:43180
```

## 主要页面

| 页面 | 内容 |
|------|------|
| **总览** | 项目统计、运行中会话、最近事件（SSE）、数据健康警告 |
| **项目** | 会话列表、事件筛选、门禁、重建索引、项目健康 |
| **对话** | 按项目浏览会话列表 + 只读事件线程（聊天式布局） |
| **会话** | Goal 运行检视、信任完成度、逐门禁状态条、时间线、产出物 |
| **报告** | 项目/会话 Markdown 导出、复制、下载 |
| **审计** | 低风险操作记录（重建索引、报告、Skills 重扫） |
| **自动化** | 读取 `~/.anycode` 下 cron 账本 |
| **Agent / Skills** | Agent 角色卡片 + 本地 SKILL.md 统计 |
| **资产** | FileWrite/Edit/Notebook 路径；支持导出 CSV |
| **设置** | 分栏：登录、数据源、服务绑定/偏好、模型路由（只读）、Skills、资产策略、安全、通知、Doctor |

侧栏含 **工作区卡片**、**导航数量徽标** 与全局搜索。总览页有 **洞察卡片**（自动化健康度、风险、建议）及 **SSE** 连接状态。事件可进入 **`/events/{id}`** 详情页。时间线行可点 **详情** 展开正文与 JSON payload。顶栏提供 **语言**（中文/English）、**主题**切换、**通知**与 **SSE** 状态（断线自动退避重连）。

**登录：** 非 loopback 绑定时未登录会跳转 `/login`；`127.0.0.1` 自动信任 `local@anycode`。

**偏好设置：** 设置 → 服务与端口 可保存 host/port/DB 到 `~/.anycode/dashboard_preferences.json`，按提示重启；`anycode dashboard` 会读取已保存偏好（显式 `--host` / `--port` / `--db` 优先）。

**发版构建**（仓库根目录）：

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo build --release -p anycode --features embedded-ui
```

Release 在编译时通过 `embedded-ui` 嵌入 `dist/`。**连接器 V2 POC：** GitHub 连接器配置 `repo` 后可在设置页只读预览 open issues（Token 来自配置或 `GITHUB_TOKEN`）。**门禁运行器 V2：** 项目详情页可一键运行 `cargo test/clippy/fmt`、`npm test`、`playwright`、`flutter test` 等预设。**安装：** `./scripts/install-with-dashboard.sh`。**测试：** `cd crates/dashboard-ui && npm test && npm run test:e2e`。**下一步规划：** [`digital-workbench-next-steps-zh.md`](../../docs/digital-workbench-next-steps-zh.md) · 另见：[工作台规划 (V3)](./dashboard-planning.md)

### 常用 API

| 端点 | 用途 |
|------|------|
| `GET /api/events/{event_id}` | 单条事件详情 |
| `GET /api/sessions?status=&trusted_status=&kind=` | 筛选会话列表 |
| `GET /api/projects/{id}/stats` | 事件/门禁/会话聚合 |
| `GET /api/artifacts?unverified_only=&blocked_session_only=` | 按信任状态筛资产 |
| `GET /api/projects/{id}/report` | 项目 Markdown/JSON 报告 |
| `GET /api/sessions/{id}/report` | 会话 Markdown/JSON 报告 |
| `GET /api/audit/events` | 看板审计日志 |
| `GET /api/settings/policies` | 本地安全策略摘要 |
| `GET /api/settings/data-health` | 数据库/项目健康检查 |
| `GET /api/settings/runtime` | LLM 配置摘要与 auth/SSE 路径 |
| `GET/PUT /api/settings/preferences` | Dashboard 绑定/DB 偏好 + 资产严格模式 |
| `PUT /api/settings/llm` | 写入 `~/.anycode/config.json` 的 LLM/备用模型 |
| `GET /api/metrics/timeline?days=7` | 近 N 日会话/事件时序 |
| `GET /api/metrics/usage?days=7` | LLM Token 汇总 + 估算 USD 成本 |
| `GET /api/metrics/usage/export?days=7` | CSV 导出（可选 `project_id`） |
| `GET /api/projects/{id}/usage?days=7` | 项目级 Token 用量 |
| `GET /api/projects/{id}/gates/presets` | 项目根目录检测到的 test/lint 预设 |
| `POST /api/projects/{id}/gates/execute` | 运行预设或自定义命令 |
| `GET /api/settings/connectors/{id}/github/issues` | GitHub open issues 只读预览 |
| `GET /api/notifications/recent` | 通知 feed |
| `GET /api/settings/connectors` | 连接器配置（UI 只读） |
| `DELETE /api/settings/notifications/{id}` | 删除通知策略 |
| `PATCH /api/settings/notifications/{id}/enabled` | 启用/禁用通知策略 |
| `POST /api/skills/{id}/all-projects` | 在所有项目上启用/禁用 Skill |
| `GET /api/auth/me` | 当前用户（loopback 自动登录） |

## 报告

侧栏进入 **报告**，或在项目/会话详情页点击 **生成报告**（支持 `?project_id=` / `?session_id=` 深链）。

报告含摘要、信任状态、门禁、产出文件、失败项与复现提示。可复制 Markdown 或下载 `.md`，不会写入代码仓库。

## 审计

执行 **重建索引**、**Skills 重扫** 或 **生成报告** 后，可在 **审计** 页查看记录。V1 操作者固定为 `local`。

## 数据健康

**设置 → 数据健康** 展示 DB 大小、孤儿引用、缺失项目根、过期运行会话、门禁/信任不一致等。**总览** 与 **项目详情** 在有问题时显示紧凑警告。

## 重建索引

在项目详情页点击 **重建索引**，会导入工作区历史任务日志并重新扫描 Skills。对早已存在、刚开启看板的项目建议执行一次。

## 环境变量

| 变量 | 作用 |
|------|------|
| `ANYCODE_DASHBOARD_RECORD` | `0` 关闭 CLI 写入（默认开启） |
| `ANYCODE_DASHBOARD_URL` | 写入后通知 SSE 的地址 |
| `ANYCODE_DASHBOARD_STATIC` | 覆盖内置 UI 目录 |
| `ANYCODE_BUILD_DASHBOARD_UI` | `1` — 在 `cargo build` 时构建 UI |
| `ANYCODE_DASHBOARD_INPUT_USD_PER_M` | 输入 Token 单价（$/M，默认 3） |
| `ANYCODE_DASHBOARD_OUTPUT_USD_PER_M` | 输出 Token 单价（$/M，默认 15） |

子命令参数：`--host`、`--port`、`--db`、`--static-dir`、`--open`。省略 `--host` / `--port` / `--db` 时使用 `~/.anycode/dashboard_preferences.json` 中已保存的值。
