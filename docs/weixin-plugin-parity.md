# 微信：OpenClaw 插件 vs anyCode Rust 桥

对照上游 **`@tencent-weixin/openclaw-weixin@2.4.3`**（OpenClaw catalog，见 OpenClaw CHANGELOG 2026.5.14/5.17）与 anyCode 原生桥 [`crates/cli/src/wx/`](../crates/cli/src/wx/)。

## 架构差异

| 项 | OpenClaw | anyCode |
|----|----------|---------|
| 运行时 | Node Gateway + npm 插件 | Rust `anycode channel wechat` / LaunchAgent |
| 调度 | Gateway 内嵌 cron / 多通道出站 | 内嵌 [`scheduler.rs`](../crates/cli/src/scheduler.rs) + [`cron_notify.rs`](../crates/cli/src/wx/cron_notify.rs) |
| 工具进度 | 依通道策略（Telegram draft 等） | **不推送** `🔧/✓` 行（仅最终回复 + 审批 + 定时提醒） |

## 2.4.3 相对 2.4.1（npm / OpenClaw #81730）

| 项 | 插件 / Gateway | anyCode Rust 桥 |
|----|----------------|-----------------|
| 目录默认版本 | `@tencent-weixin/openclaw-weixin@2.4.3` 替换 2.4.1 | 无 npm；以本文件 + `bridge.rs` diff 跟踪 |
| 完整性 pin | `openclaw channels add` 带 package integrity | N/A |
| 插件 SDK 兼容 | `normalizeAccountId`、`resolvePreferredOpenClawTmpDir`（#53497） | 非插件运行时；账号 id 在 `wx` 配置与 session 路径 |

## 建议对齐项（按优先级）

| 优先级 | OpenClaw / 插件能力 | anyCode 现状 | 动作 |
|--------|---------------------|--------------|------|
| P1 | 入站正文 `bodyFromItemList`、首段 TEXT、引用 `ref_msg` | CHANGELOG 已对齐过一版 | 随插件 2.4.x 发版 diff `bridge.rs` / `wechat_ilink.rs` |
| P1 | 媒体优先级 IMAGE > VIDEO > FILE > VOICE | 桥接层有 fallback | 对照插件解密/CDN 变更 |
| P2 | 群聊诊断 / 未注册群提示（WhatsApp 5.19 同类） | 部分日志 | 改进 `wx.ftl` 与日志，不阻塞回复 |
| P2 | 出站队列：连接时已排队消息需定时 drain（WhatsApp 5.19） | iLink `send_text` 直连 | 评估发送失败重试与队列 |
| P2 | 多账号 / `normalizeAccountId` 迁移 | 单账号 LaunchAgent 为主 | 文档化多 profile 非目标，除非用户要 parity |
| P3 | 插件 SDK 根别名桥 | N/A（非插件） | 仅当改共享协议时参考 |

## 跟踪方式

1. 查看 npm：<https://www.npmjs.com/package/@tencent-weixin/openclaw-weixin> 的 **版本** 与 **CHANGELOG**（对标基线 **2.4.3**，2026-05-18 核对）。
2. 与 OpenClaw 仓库 `CHANGELOG.md` 中 `Channels/Weixin` 条目交叉核对。
3. 差异记入本文件；需改代码的项开 GitHub issue，并在 [roadmap.md](roadmap.md) §4 引用。

## 相关

- [openclaw-sync-brief-2026-05.md](openclaw-sync-brief-2026-05.md)
- [wx-streaming-bridge.md](wx-streaming-bridge.md)
