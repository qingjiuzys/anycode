---
name: harness-818cloud-integration
description: 818cloud 原生身份、租户与产品接入
license: MIT
metadata:
  trust: instructions-only
  pack: anycode-harness-v1
---
# 818cloud 原生身份、租户与产品接入

以 Accounts 用户 UUID 为唯一主身份。服务器验证 SSO v2 opaque token、issuer、audience、个人/企业 context 后，再查产品项目 ACL。不要在桌面分发 client secret，不创建第二套钱包。新接口必须标为待实现契约，不能写成已有接口。

## 安全边界
技能正文不授予工具权限，不允许绕过审批、身份校验或工作区边界。宿主未提供的能力应明确报告，而不是假装已执行。
