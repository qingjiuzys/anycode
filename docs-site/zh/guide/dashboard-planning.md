---
title: 数字工作台 — 规划
description: V1+V2 完成状态与 V3 路线图入口。
---

# 数字工作台 — 规划

**状态：** V1 MVP + V2 切片 A–D 已完成（2026-05），适用于本地单用户。

决定下一步做什么时，从本文开始。完整细节在仓库 `docs/` 目录。

## 已完成

| 层级 | 内容 |
|------|------|
| CLI | `anycode dashboard`、doctor、status、token、db backup |
| 数据 | run/goal/workflow/repl/cron → SQLite |
| 信任 | 门禁阻断；无门禁且已完成 → verified |
| UI | React/Vite、中英文、SSE、12+ 页面、release 内嵌 UI |
| V2-A | 项目 Token、CSV 导出、blocked 告警 |
| V2-B | GitHub open issues 只读（设置 + 自动化） |
| V2-C | Gate Runner（预设、执行、入库） |
| V2-D | 安装脚本、文档、11 项 Playwright e2e |

## 自检

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## 仓库文档（维护者）

| 文档 | 用途 |
|------|------|
| [digital-workbench-next-steps-zh.md](https://github.com/qingjiuzys/anycode/blob/main/docs/digital-workbench-next-steps-zh.md) | **规划入口** — V3 分级 + 示例四周路线 |
| [digital-workbench-handoff-zh.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-handoff-zh.md) | 交接摘要 |
| [digital-workbench-v2-complete.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-v2-complete.md) | V2 清单 |
| [digital-workbench-v1-mvp.md](https://github.com/qingjiuzys/anycode/blob/main/docs/archive/workbench/digital-workbench-v1-mvp.md) | V1 UX 验收 |

用户指南：[数字工作台](./dashboard.md)。

## V3 — 尚未实现

### 第一梯队（本地、价值高）

- 按 provider/model 成本
- 节省工时 KPI
- Gate 运行历史与流式输出
- Linear 只读 Connector
- 生产部署清单

### 第二梯队（控制面）

- UI 取消运行中会话
- Web 触发 run
- 工具审批收件箱

### 第三梯队（多用户/外部）

- SSO/RBAC、Connector 写入、Browser gate、Tauri

## 规划四问

1. 下一用户是谁？（个人 / 团队 / CI）
2. 核心指标？（成本 / 信任 / 吞吐）
3. Connector 是否够用？
4. 是否要从 Web 控制 Agent？

详见 [next-steps-zh](https://github.com/qingjiuzys/anycode/blob/main/docs/digital-workbench-next-steps-zh.md)。
