---
name: harness-graph-authoring
description: 版本化工作流图编写与验收
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 版本化工作流图编写与验收

使用 version=1 的有向无环图。条件分支仅读取声明的前驱输出；合流使用明确 all/any。验收用 Gate，人工决定用 Human。Partial、Failed、Uncertain 不能被标记 Completed。此版本不支持任意 LangGraph Python 程序或循环图。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
