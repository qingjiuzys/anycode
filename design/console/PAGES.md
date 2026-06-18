# 818Cloud Console — 页面规格

> 逐页 UX 规格。套餐：Free / Pro / Team；计费：monthly / yearly。  
> 视觉见 [VISUAL_SYSTEM.md](./VISUAL_SYSTEM.md)。

---

## 全局 Shell

侧栏 260px + 主内容区。侧栏底部额度卡：

```
Pro · 月付
[██████░░░░░░] 52%
账期剩余 12 天 · 2.6M / 5M tokens
```

点击 → `section=usage`

---

## `overview` — 控制台总览

### KPI 卡片（4 列 → 移动端 1 列）

| 卡片 | 主值 | 副标签 | 跳转 |
|------|------|--------|------|
| 当前套餐 | Pro | 月付 · 生效中 | `plan` |
| Token 用量 | 2.6M / 5M | 账期剩余 12 天 | `usage` |
| API Keys | 2 / 5 | 账户接口密钥 | `api` |
| 团队席位 | 3 / 10 | Team 套餐 | `enterprise` |

### 快捷操作

- 升级套餐 → `plan`
- 查看账单 → `billing`
- 管理 API Keys → `api`

### 副标题

「{plan} 套餐生效中，当前账期 {start} — {end}。」

**不展示**：网关延迟、滚动窗口、假验证码。

---

## `usage` — 用量

1. 本账期 Token 配额进度 + 剩余天数
2. ≥80% 升级提示卡
3. 30 天 KPI：调用、输入/输出 tokens、估算成本
4. 按模型表 + CSV 导出
5. 7 天趋势图

额度用尽文案：「本账期 Token 配额已用尽，将于 {end} 重置或升级套餐。」

---

## `plan` — 套餐

三卡：Free / Pro（featured）/ Team

- 顶栏：**月付 | 年付** 切换
- 价格：月付 `$29/月`，年付 `$290/年`（示例）
- Team CTA：联系销售
- 生产环境：无「模拟升级」按钮

---

## `billing` — 账单

- 当前账期、月付/年付、预估金额、状态
- 发票表
- 账单联系人表单
- 支付方式：已绑定 / 即将推出

---

## `api` — API Keys

说明：「账户 API Key，非 LLM 推理密钥。」

- 创建 → 一次性 plaintext
- 撤销 → 确认模态

---

## `enterprise` — 团队

**Team 用户**：组织信息、seats、成员表、能力卡片

**非 Team**：EmptyState +「查看 Team 套餐」

邀请成员：disabled +「即将推出」

---

## 生产红线

1. 无 Cloud 5h / 滚动窗口 / 1000 次文案
2. 无 mock 升级（生产）
3. 无 fake 网关指标
4. 默认 section=overview
