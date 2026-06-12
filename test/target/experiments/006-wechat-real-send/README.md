# 006 - WeChat Real Send

向最近一次微信会话发送真实测试消息，验证 WeChat bridge 的发送链路和 outbound ledger。

## 前置条件

1. 微信桥接已绑定并在运行（`anycode channel status wechat` 均为 ok）
2. **测试前请先在微信里给 bot 发一条消息**，刷新 `cron_notify_target.json` 里的 `contextToken`；长时间无对话时 iLink 会以 `ret=-2` 拒绝发送

运行：

```bash
node test/target/experiments/006-wechat-real-send/run.mjs
```

该实验会产生真实微信消息。消息带有 `[anycode-e2e:<run_id>]` marker，便于识别和核验。若发送失败，ledger 会出现 `failed` 记录，实验不通过。
