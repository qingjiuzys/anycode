---
name: harness-release-review
description: 发布前检查、补丁审阅与回滚
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 发布前检查、补丁审阅与回滚

只读审阅 diff、依赖锁、迁移、权限变化和测试结果。发布、推送镜像、合并和切流是不同操作，必须分别授权。没有真实生产验证不能标记已上线。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
