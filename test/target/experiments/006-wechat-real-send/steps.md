1. 读取 `anycode channel status wechat --json`。
2. 确认 data_dir、cron_target、scheduler 为 ok。
3. 读取 redacted 的 outbound ledger 快照。
4. 调用 `anycode channel wechat-send-test --message <固定消息> --json`。
5. 再次读取 outbound ledger。
6. 校验同一 marker 出现 pending 和 sent 记录。
