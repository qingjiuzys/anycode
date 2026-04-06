---
title: 路由
description: 通过 config.json 的 routing.agents 按 agent 类型覆盖模型与端点。
summary: plan / explore / summary 与全局 model 的优先级。
read_when:
  - 要为规划与探索使用不同模型。
  - 首次编辑 routing.agents。
---

# 智能路由（按工作类型选模型）

anyCode 允许按 `agent_type` 覆盖模型与端点，从而实现：

- 规划（plan）用更强模型
- 探索（explore）用更快/更便宜模型
- 总结（summary）用单独 profile（或复用 plan）

## 配置示例

编辑 `~/.anycode/config.json`：

```json
{
  "routing": {
    "agents": {
      "plan": { "model": "glm-5", "plan": "general" },
      "explore": { "model": "glm-4.7", "plan": "coding" },
      "summary": { "model": "glm-5", "plan": "general" }
    }
  }
}
```

## 优先级

1. `routing.agents.<agent_type>`
2. summary 阶段：`routing.agents.summary` → `routing.agents.plan` → default
3. 全局 `model/plan/base_url`

English: [Routing](/guide/routing).

