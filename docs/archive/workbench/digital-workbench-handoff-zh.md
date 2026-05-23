# Digital Workbench — 交接与下一步规划

**状态：** V1 MVP + **V2 切片 A–D 已完成**（2026-05）。一页状态：[`digital-workbench-STATUS.md`](digital-workbench-STATUS.md)。

清单：[`digital-workbench-v1-mvp.md`](digital-workbench-v1-mvp.md)、[`digital-workbench-v2-complete.md`](digital-workbench-v2-complete.md)。英文详情见 [`digital-workbench-handoff.md`](digital-workbench-handoff.md)。

## V2 已交付摘要

| 切片 | 内容 |
|------|------|
| **A 观测** | 项目级 Token/成本、CSV 导出、blocked 超阈值告警 |
| **B Connector** | GitHub open issues 只读预览（设置 + 自动化页） |
| **C Gate Runner** | UI 运行 test/lint 预设并写入 gates/时间线；goal 引擎已有真实 cargo/flutter 校验 |
| **D 打包** | `install-with-dashboard.sh`、`--with-dashboard`、文档更新 |

## 自检

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## V3+ 未做（路线图）

Connector OAuth/写入、SSO/RBAC、UI 控制 Agent、Browser gate 自动化、Tauri、节省工时 KPI、按 provider 定价 — 见英文 handoff Effort 表。

## 建议 V3 方向

1. **控制面** — 只读 → 受控操作（取消 run、重跑 gate）
2. **Connector 深化** — OAuth、issue 与会话关联
3. **成本精度** — 模型价目表 + 会话级拆分
4. **生产部署** — 反向代理、TLS、备份自动化

**规划入口：** [`digital-workbench-next-steps-zh.md`](digital-workbench-next-steps-zh.md)（英文: [`digital-workbench-next-steps.md`](digital-workbench-next-steps.md)）
