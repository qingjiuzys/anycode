# 818Cloud Console — 设计总览

> **目标站点**：`http://console.818cloud.com/`  
> **参考原型**：[`818cloud-ui-prototype.html`](../../../anycode/818cloud-ui-prototype.html)（控制台布局与信息层级）  
> **视觉基调**：暗夜电紫 · 毛玻璃 · 生产级控制平面  
> **状态**：设计规范 v1.1（2026-06）

---

## 1. 设计目标

818Cloud Console 是 anyCode 的**云端账户控制平面**，不是营销落地页。用户登录后应能在 10 秒内回答四个核心问题：

| # | 用户问题 | 首屏必须呈现 |
|---|----------|--------------|
| 01 | 我现在能不能用？ | 订阅状态、账户在线、服务可用性 |
| 02 | 我还有多少额度？ | 当前账期 token 配额、剩余天数 |
| 03 | 我的设备连上了吗？ | 设备绑定入口（接口就绪后） |
| 04 | 下一步该做什么？ | 升级套餐、查看发票、管理 API Key |

**套餐模型（收敛）**：

- 仅 **Free / Pro / Team** 三档。
- 付费仅 **月付（monthly）/ 年付（yearly）**。
- **不包含** Cloud 5h、滚动窗口、按 5 小时重置的调用次数配额。

**设计原则**：

1. **控制台优先**：登录后直达总览，套餐对比下沉到独立页面。
2. **状态可信赖**：所有额度/账单状态使用明确文案 + 颜色 + 图标，禁止 mock 占位。
3. **品牌一致**：复用 [`design/glass-skins`](../glass-skins/TOKENS.md) 电光蓝紫皮肤，强化暗色毛玻璃质感。
4. **生产就绪**：覆盖空态、加载态、错误态、权限不足、支付中断等真实场景。
5. **中英双语**：所有用户可见文案走 i18n key，禁止 key 泄漏。

---

## 2. 目标用户

| 角色 | 典型场景 | 核心页面 |
|------|----------|----------|
| 个人开发者 | 绑定本机 anycode Desktop，使用 Pro 托管模型 | 总览、用量 |
| 付费用户 (Pro) | 管理月/年 token 额度、查看账单、配置 API Key | 用量、套餐与账单、API Keys |
| 团队管理员 (Team) | 邀请成员、分配 seats、统一发票 | 团队权限、套餐与账单 |

---

## 3. 产品边界

### 在 Console 内

- 账户登录 / 登出（云端门户）
- 套餐选择、升级（月付/年付）
- 账单、发票、支付方式
- 当前账期 token 用量与导出
- 账户 API Keys（非 LLM Key）
- 团队成员与角色（Team 套餐）

### 不在 Console 内（Phase 2+）

- 设备绑定验证码流程（接口就绪后接入）
- 模型网关实时指标（无 API 时不展示假数据）
- Agent 工具执行、会话管理 → anycode Desktop / Workbench

---

## 4. 页面地图

```
console.818cloud.com  （或 Workbench /account）
├── /login                    登录
├── /account?section=overview 控制台总览（默认）
├── /account?section=usage    用量日志
├── /account?section=plan     套餐对比与升级
├── /account?section=billing  账单与发票
├── /account?section=api      API Keys
└── /account?section=enterprise 团队
```

---

## 5. 与现有代码的映射

| 现有组件 | Console 页面 | 改造要点 |
|----------|--------------|----------|
| `ConsoleShell` | 全局 Shell | 左侧导航 + 额度卡片 + 顶栏用户区 |
| `ServiceOverviewSection` | `overview` | 套餐、账期、token/API key/seats KPI |
| `ServicePlanSection` | `plan` | 月付/年付切换；生产环境隐藏 mock 升级 |
| `ServiceUsageSection` | `usage` | 按账期 token quota，无滚动窗口 |
| `ServiceBillingSection` | `billing` | 账期、发票、联系人 |
| `ServiceApiSection` | `api` | 区分账户 Key vs LLM Key |
| `ServiceEnterpriseSection` | `enterprise` | Team 门控与升级引导 |

API 类型：[`accountCloud.ts`](../../crates/dashboard-ui/src/api/types/accountCloud.ts)、[`planCatalog.ts`](../../crates/dashboard-ui/src/lib/planCatalog.ts)。

---

## 6. 文档索引

| 文档 | 内容 |
|------|------|
| [VISUAL_SYSTEM.md](./VISUAL_SYSTEM.md) | 暗夜电紫设计令牌、毛玻璃组件 |
| [INFORMATION_ARCHITECTURE.md](./INFORMATION_ARCHITECTURE.md) | 导航、数据依赖、路由 |
| [PAGES.md](./PAGES.md) | 逐页 UX 规格 |
| [PRODUCTION_CHECKLIST.md](./PRODUCTION_CHECKLIST.md) | 上线验收清单 |

---

## 7. 实施阶段

### Phase 1 — Shell + 总览

- Console Shell（侧栏、额度卡、玻璃风格）
- 总览页 KPI（套餐、账期、token、API keys、seats）

### Phase 2 — 套餐 / 账单 / 用量

- Free/Pro/Team + monthly/yearly
- 账期 token 用量与升级提示
- 生产环境去除 mock 升级文案

### Phase 3 — API Keys / 团队 + 生产加固

- API Key 撤销确认、Team 门控
- i18n、响应式、验收清单

---

## 8. 参考

- [ADR 011 — Cloud account platform](../../docs/adr/011-cloud-account-platform.md)
- [ADR 012 — WeChat Pay prepaid billing](../../docs/adr/012-wechat-pay-prepaid-billing.md)
- [Glass Skins TOKENS](../glass-skins/TOKENS.md)
