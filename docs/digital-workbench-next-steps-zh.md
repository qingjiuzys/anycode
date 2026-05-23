# Digital Workbench — 下一步规划

**当前位置：** V1 MVP + V2 + **V3 Week 1–10** 已完成。下一步建议先做 **[Production Harness Hardening](production-harness-hardening.md)**：在进入 Connector 写回、SSO/RBAC 或桌面壳之前，补齐执行轨迹、运行时预算、轨迹评估、工具/MCP 治理、声明式 workflow 校验与记忆治理。

## 已有能力

| 领域 | 说明 |
|------|------|
| CLI | `anycode dashboard`（doctor / status / token / db backup） |
| 录制 | run / goal / workflow / repl / cron → SQLite + SSE |
| 信任 | 门禁阻断交付；无门禁且已完成 → verified |
| UI | 12+ 页面、中英文、懒加载、release 内嵌 UI |
| 观测 | 全局/项目/会话 Token、CSV 导出、时间线、就绪度 |
| 告警 | blocked 超阈值 → `blocked_threshold_exceeded` |
| Connector | GitHub + Linear open issues 只读 |
| Gate Runner | UI 预设 → shell → gates 表 + **SSE 流式日志** |
| 控制面 | 会话取消（DB + live IPC）、**UI 触发 run/goal**、gate required |
| 安全收件箱 | **交互式 Web 审批**（首页 + 运行中会话详情）+ 历史日志 |
| Goal | 引擎真实 cargo/flutter 校验；`[gate]` 日志入库 |
| 安装 | `./scripts/install-with-dashboard.sh` |
| 测试 | 59 项 Rust dashboard + 28 项 Playwright e2e |

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
| [`archive/workbench/digital-workbench-closure-report.md`](archive/workbench/digital-workbench-closure-report.md) | Control-plane closure summary |
| [`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) | 一页状态 |
| [`digital-workbench-control-plane.md`](digital-workbench-control-plane.md) | 控制面行为说明 |
| [`digital-workbench-deploy-production.md`](digital-workbench-deploy-production.md) | 生产部署 |
| [`digital-workbench-api.md`](digital-workbench-api.md) | API 合约 |
| [`production-harness-hardening.md`](production-harness-hardening.md) | Tier 1.5 生产级 Harness 加固路线 |

---

## Tier 1.5 — Production Harness Hardening

该阶段位于 V3 控制面之后、Tier 2/3 企业化能力之前。目标是在不引入第二套执行引擎的前提下，让 `AgentRuntime` 继续作为唯一编排权威，并把 Digital Workbench 从“可观测控制面”升级成“可治理 Agent Harness”。

| 优先级 | 项 | 工作量 | 结果 |
|--------|----|--------|------|
| P0 | **Execution trace SSOT** | L | 结构化 task/turn/LLM/tool/gate/budget 事件，支撑 replay、eval、audit 与 provenance |
| P0 | **Runtime budget** | L | token/cost/duration 预算在运行中 warning / degrade / hard-stop |
| P0 | **Trajectory eval** | M | CI 能抓重复工具、禁用工具、gate 失败、预算违规，即使最终文本看似成功 |
| P1 | **Tool governance metadata** | M | Tool catalog 记录风险等级、类别、审批策略、Agent 可见性与审计级别 |
| P1 | **MCP governance** | M | 可选 strict 白名单、per-server 配额与 MCP trace |
| P1 | **Declarative workflow validation** | M–L | Planner 只产计划；Harness 在执行前校验 agent、tool、gate、budget 与依赖 |
| P2 | **Memory retention** | M | 热层/向量记忆支持 dry-run prune、retention score 与 evidence provenance |
| P2 | **Workbench operations UI** | M | Dashboard 解释预算健康、轨迹回放、轨迹评估、工具风险与记忆治理 |

推荐顺序：先做 trace，再做 runtime budget，再做 trajectory eval。这三项是后续工具治理、MCP 治理、声明式 workflow 和记忆治理的共同基础。

## Tier 2 — 控制面（需安全设计）

| 项 | 工作量 | 说明 |
|----|--------|------|
| Connector OAuth / 写入 | L | GitHub/Linear PR、issue 创建 |

## Tier 3 — 多用户 / 外部系统

| 项 | 工作量 | 说明 |
|----|--------|------|
| SSO / OIDC | L | 非 loopback 多用户前置 |
| RBAC | L | 落实 permissions 文档角色 |
| Connector OAuth/写入 | L | GitHub/Linear 写回 |
| Browser gate | M–L | 无头视觉验收 |
| Tauri 桌面 | L | 包装 embedded UI |

---

## 规划前先答四问

1. 下一用户是谁？（个人 / 小团队 / CI 只读集成）  
2. 核心指标？（成本 / 信任门禁 / 吞吐）  
3. Connector 是否够用？（GitHub/Linear 只读 vs 写回/Slack）  
4. 是否要从 Web **控制** Agent？（仅观测 vs 受控操作）  

答完后从 Tier 2 表拆 issue 即可。
