---
name: harness-subagent-delegation
description: 子代理任务拆解与隔离
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 子代理任务拆解与隔离

为子代理提供窄任务、必要上下文和明确输出格式。权限只能减少，预算来自同一根账本；独立写任务使用独立 worktree。禁止通过旧 Agent/Task 工具另起不受监管的循环。容量不足应收敛调度，不递归无限重试。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
