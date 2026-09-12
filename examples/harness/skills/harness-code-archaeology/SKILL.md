---
name: harness-code-archaeology
description: 代码调研与调用链定位
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 代码调研与调用链定位

先读 AGENTS.md、Cargo.toml、composition root、真实入口和测试。记录 commit SHA、路径、符号及最小证据。区分已有能力、未接线、已验证问题和推测。不要把 README 的宣称当成运行证明。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
