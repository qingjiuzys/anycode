---
name: harness-rust-refactor
description: Rust 分层重构与兼容迁移
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# Rust 分层重构与兼容迁移

先定义行为契约和回归用例；一次只迁移一个入口。保留既有安全管线、消息元数据、取消语义和工具错误语义。运行 cargo fmt、check、test、clippy；无法运行时准确报告原因。不得为通过测试删除断言或返回假成功。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
