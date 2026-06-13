# Digital Workbench — 下一步规划

**当前位置：** V1 MVP + V2 + **V3 Week 1–10** 已完成（本地控制面）。下一步 **0.3** 主线：**网页账号控制台** — 登录、套餐/订阅、用量、账单、API 管理、企业能力入口。Agent **执行仍在 CLI / 本地 runtime**；**0.3 不做网页端操作 Agent**。详见 [`roadmap.md`](../roadmap.md) §3.5。

## 已有能力

| 领域 | 说明 |
|------|------|
| CLI | `anycode dashboard`（doctor / status / token / db backup） |
| 录制 | run / goal / workflow / repl / cron → SQLite + SSE |
| 信任 | 门禁阻断交付；无门禁且已完成 → verified |
| UI | 12+ 页面、中英文、懒加载、release 内嵌 UI |
| 认证（本地） | `/login`、`/api/auth/*`、loopback `local_trusted` |
| 观测 | 全局/项目/会话 Token、CSV 导出、时间线、就绪度 |
| 告警 | blocked 超阈值 → `blocked_threshold_exceeded` |
| Connector | GitHub + Linear open issues 只读 |
| Gate Runner | UI 预设 → shell → gates 表 + **SSE 流式日志** |
| 控制面（本地） | 会话取消、UI 触发 run/goal、Web 工具审批、Conversations 工作流 |
| Goal | 引擎真实 cargo/flutter 校验；`[gate]` 日志入库 |
| 安装 | `./scripts/install-with-dashboard.sh` |
| 测试 | 69+ 项 Rust dashboard + 28 项 Playwright e2e |

**自检命令：**

```bash
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
cargo test -p anycode-dashboard
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
anycode dashboard --open
```

## 文档索引

| 文档 | 用途 |
|------|------|
| [`../roadmap.md`](../roadmap.md) | **0.3 SSOT** — 网页控制台 A–E 包 |
| [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) | 一页状态 |
| [`digital-workbench-api.md`](digital-workbench-api.md) | API 合约（auth 模式） |
| [`digital-workbench-permissions.md`](digital-workbench-permissions.md) | 角色与企业模式 |
| [`production-harness-hardening.md`](../planning/production-harness-hardening.md) | **0.4** Harness 加固（非 0.3） |
| [`digital-workbench-deploy-production.md`](digital-workbench-deploy-production.md) | 生产部署 |

---

## 0.3 — 网页账号控制台

产品壳参考 SaaS 后台：**套餐管理、用量管理、账单与发票、API 管理、账号设置、企业/组织**。订阅/账单可先用 **mock 数据**，0.3 不绑真实支付。

| 优先级 | 项 | 工作量 | 结果 |
|--------|----|--------|------|
| P0 | **网页登录与会话** | M | 邮箱/密码或 token 登录；用户菜单；Settings → Auth；loopback 保留 `local_trusted` |
| P0 | **套餐 / 订阅壳** | M | 套餐展示、订阅状态、升级入口（可 mock） |
| P0 | **用量管理** | M | 将现有 token/cost 指标包装为用户「用量」页；配额/超限提示 |
| P1 | **账单与发票壳** | M | 账单列表、下载占位、开票信息（无真实支付） |
| P1 | **API 管理** | M | API key 创建/撤销/轮换；最近使用与 scope |
| P1 | **企业管理壳** | L | 组织、成员、角色、审计入口；SSO/OIDC **仅设计占位** |

推荐顺序：**0.3-A → B+C → D → E**（与 [`roadmap.md`](../roadmap.md) §3.5.1 一致）。

### 0.3 明确不做

- **网页端操作 Agent**（以产品能力承诺 Web 触发 run、工具审批、会话 cancel 等）。
- 远程任务队列、云端 Agent 运行时、OpenClaw Gateway 式 relay。
- 真实支付网关（Stripe、微信支付等）。

V3 本地控制面能力保留给 **loopback 开发**；不作为 0.3 产品扩展方向。

---

## 0.4 — Production Harness Hardening（延后）

执行轨迹、运行时预算、轨迹 eval、工具/MCP 治理 — 见 [`production-harness-hardening.md`](../planning/production-harness-hardening.md)、[`closure-plan-2026-06.md`](../planning/closure-plan-2026-06.md)。Epic 映射见 [`roadmap.md`](../roadmap.md) §4。

---

## 更晚（0.3 之后）

| 项 | 工作量 | 说明 |
|----|--------|------|
| Connector OAuth / 写入 | L | GitHub/Linear 写回 |
| 完整 SSO / OIDC | L | 超出 0.3 设计占位 |
| RBAC 强制 enforcement | L | 落实 permissions 文档 |
| Browser gate | M–L | 无头视觉验收 |
| 真实计费集成 | L | 订阅壳完成后再做 |

---

## 规划前先答四问

1. **0.3 侧栏 IA？** 套餐 · 用量 · 账单 · API · 账号 · 企业（参考 CODEBUDDY 式后台）。  
2. **权益模型？** Free / Pro / Team；按 token 配额还是按席位。  
3. **远程 bind 认证？** 仅 API token vs 邮箱密码 + session cookie。  
4. **网页是否操作 Agent？** 0.3 默认 **否** — CLI 仍是执行面。  

答完后从 [`roadmap.md`](../roadmap.md) §3.5 拆 issue。
