# 点对点系统测试

`test/target` 是 anyCode 的点对点测试工作区。每个实验目录都是一个独立场景，包含固定材料、提示词、步骤、期望输出、自动化脚本和结果产物。

## 一键运行

```bash
node test/target/run_all.mjs
```

默认测试本地 Dashboard：

```text
http://127.0.0.1:43180
```

可通过环境变量覆盖：

```bash
ANYCODE_DASHBOARD_URL=http://127.0.0.1:5174 node test/target/run_all.mjs
```

## 实验列表

| ID | 场景 |
| --- | --- |
| `001-skills-office-export` | Skills 办公报表导出，CSV + PDF |
| `002-skills-readonly-db` | SQLite 只读分析边界 |
| `003-web-workbench` | Dashboard 首页工作台 |
| `004-web-agents-skills` | Agent / Skills 页面 |
| `005-macos-client-channels` | macOS 客户端消息渠道设置 |
| `006-wechat-real-send` | 微信真实自动发送与 outbound ledger |

## 输出

- 每个实验：`experiments/<id>/out/result.json`
- 总报告：`out/summary.json`、`out/summary.md`

## 注意

`006-wechat-real-send` 会发送一条真实微信测试消息，消息包含 `[anycode-e2e:<run_id>]` marker。报告会 redacted 敏感字段，不输出 token、user id 或 context token。
