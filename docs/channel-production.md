# Channel Production Reliability

The production channel target is clear state, recoverable delivery, and useful
diagnostics across WeChat, Telegram, and Discord.

## Diagnostics

- `anycode channel status [wechat|telegram|discord|all]`
- `anycode doctor channel [wechat|telegram|discord|all]`

The status output checks credential files, WeChat data directories, cron notify
target state, and scheduler lock hints.

## State Machine Target

```text
Idle -> Processing -> Idle
Idle -> WaitingPermission -> Processing
Idle -> WaitingQuestion -> Processing
Processing -> Cancelled -> Idle
```

Only one wait state should be active per chat. Telegram already has structured
`AskUserQuestion`; Discord and WeChat should add host implementations in later
slices per ADR 008.

## Delivery Target

Outbound queues should eventually record:

- pending
- sent
- failed
- retry_count
- last_error

WeChat already has transient `send_text` retry; the remaining production work is
queue persistence and channel-wide status display.

