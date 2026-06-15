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
| P2 | 出站队列：连接时已排队消息需定时 drain（WhatsApp 5.19） | iLink `send_text` 直连 | **Done**（2026-05-19）：`send_text` 瞬态 HTTP 重试 + bridge 记录 chunk 失败 |
| P2 | 入站媒体 IMAGE>VIDEO>FILE>VOICE | `cdn_media::select_inbound_media_item` | **Done**（2026-05-20）：`select_video_before_file` 单测锁定 VIDEO>FILE |
| P2 | CDN 拉取 URL 白名单 | `cdn_get_url_trusted` | **Done**（2026-05-20）：仅 `*.weixin.qq.com` / `*.wechat.com` 等 |
| P2 | 多账号 / `normalizeAccountId` 迁移 | 单账号 LaunchAgent 为主 | 文档化多 profile 非目标，除非用户要 parity |
| P1 | 出站媒体：`sendWeixinMediaFile`（image/video/file CDN） | `send_media.rs` + `WxSender::send_*_message` | **Done**（2026-06-15）：`base_info` + `iLink-App-*` 头、`getUploadUrl` ret 校验、CDN 错误传播 |
| P1 | 出站触发：`ReplyPayload.mediaUrl` / session attachments | `resolve_outbound_media_paths`（本地路径 + http(s) URL 下载） | **Done**（2026-06-15） |
| P1 | 工具出站媒体：`SendWeChatMessage` + `path`/`file` | `send_wechat_media` + `send_deliverable_path` | **Done**（2026-06-13）：文本/文件/说明文本，复用 bridge CDN 路由 |
| P3 | 插件 SDK 根别名桥 | N/A（非插件） | 仅当改共享协议时参考 |

## 跟踪方式

1. 查看 npm：<https://www.npmjs.com/package/@tencent-weixin/openclaw-weixin> 的 **版本** 与 **CHANGELOG**（对标基线 **2.4.3**，2026-05-18 核对）。
2. 与 OpenClaw 仓库 `CHANGELOG.md` 中 `Channels/Weixin` 条目交叉核对。
3. 差异记入本文件；需改代码的项开 GitHub issue，并在 [roadmap.md](../roadmap.md) §4 引用。

## 相关

- [openclaw-sync-brief-2026-05.md](../comparisons/openclaw-sync-brief-2026-05.md)
- [wx-streaming-bridge.md](wx-streaming-bridge.md)

## G1 出站 CDN 结论（2026-06-15）

**根因（推断）**：anyCode iLink 请求缺少 OpenClaw 2.4.3 统一的 `base_info`（`channel_version` / `bot_agent`）与 `iLink-App-Id` / `iLink-App-ClientVersion` 头；文本 `sendmessage` 可能仍成功，但 `getUploadUrl` + CDN 预签名对 wire 字段更严格，导致小体积 mp4（如 870KB）也回退为路径提示。

**已补齐**：[`ilink.rs`](../crates/cli/src/channels/wx/ilink.rs) wire parity；[`cdn_upload.rs`](../crates/cli/src/channels/wx/cdn_upload.rs) `getUploadUrl` ret 校验；[`deliverable.rs`](../crates/cli/src/channels/wx/deliverable.rs) 区分「过大」与「CDN 失败」文案 + 远程 URL 下载。

**待实机确认**：微信内收到视频消息（非路径回退）；见 [closure-plan-2026-06.md](../planning/closure-plan-2026-06.md) §4.1。
