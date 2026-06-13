---
title: 数字工作台 — 规划
description: V1–V3 完成状态与 0.3 网页控制台路线图。
---

# 数字工作台 — 规划

**状态：** V1 MVP + V2 + V3 控制面已完成（2026-05），适用于本地单用户。

**下一步（0.3）：** **网页账号控制台** — 登录、套餐/订阅、用量、账单、API 管理、企业入口。**Agent 仍在终端执行**；0.3 **不做网页端操作 Agent**。

决定下一步做什么时，从本文开始。完整细节在仓库 `docs/` 目录。

## 已完成

| 层级 | 内容 |
|------|------|
| CLI | `anycode dashboard`、doctor、status、token、db backup |
| 数据 | run/goal/workflow/repl/cron → SQLite |
| 信任 | 门禁阻断；无门禁且已完成 → verified |
| UI | React/Vite、中英文、SSE、12+ 页面、release 内嵌 UI |
| 认证（本地） | `/login`、会话 API、loopback `local_trusted` |
| V2–V3 | Token 观测、Connector、Gate Runner、本地控制面、e2e |

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
| [roadmap.md §3.5](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md) | **0.3 SSOT** — 网页控制台交付包 |
| [digital-workbench-next-steps-zh.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps-zh.md) | **规划入口**（中文） |
| [digital-workbench-api.md](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-api.md) | API 合约 |
| [production-harness-hardening.md](https://github.com/qingjiuzys/anycode/blob/main/docs/planning/production-harness-hardening.md) | **0.4** Harness（非 0.3） |

用户指南：[数字工作台](./dashboard.md)。

## 0.3 — 网页控制台（产品壳待建）

| 模块 | 目标 |
|------|------|
| 账号 | 登录、用户菜单、账号设置 |
| 套餐 | 订阅档位、升级入口（可 mock） |
| 用量 | 面向用户的 token/成本页 |
| 账单 | 发票壳（0.3 无真实支付） |
| API | Key 创建/撤销/轮换 |
| 企业 | 组织、成员、角色、审计入口 |

**0.3 不做：** 以产品能力承诺网页操作 Agent；云端 Agent 托管；真实支付网关。

## 规划四问

1. 侧栏 IA？套餐 · 用量 · 账单 · API · 账号 · 企业  
2. 权益模型？Free / Pro / Team；配额 vs 席位  
3. 远程认证？仅 API token vs 邮箱密码会话  
4. 网页是否操作 Agent？0.3 默认 **否** — 用 CLI  

详见 [next-steps-zh](https://github.com/qingjiuzys/anycode/blob/main/docs/workbench/digital-workbench-next-steps-zh.md)。
