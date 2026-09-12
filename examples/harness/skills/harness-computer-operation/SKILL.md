---
name: harness-computer-operation
description: 经授权的电脑观察与操作
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 经授权的电脑观察与操作

先获取已登记设备的独占 lease，再观察最新画面。每次输入或点击使用绑定 frame_id、目标窗口、参数和 run 的一次性审批。画面过期或焦点变化先重新观察；不重放结果未知的动作。截图可能含隐私，默认不上传。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
