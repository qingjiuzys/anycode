# 状态行（HUD）

全屏 TUI 可在 **输入区上方** 显示一条状态行，概念对齐 Claude Code 的 `statusLine`。

## 配置

在 `~/.anycode/config.json` 中增加 `statusLine` 段，字段含义见英文文档 `docs-site/guide/status-line.md`。

快速查看当前环境下 stdin JSON 样例：

```bash
anycode statusline print-schema
```

**安全提示：** `command` 会以你的用户身份执行，等价于在配置里写死一段 shell。

## 示例脚本

仓库内 `scripts/statusline-example.sh` 提供基于 `jq` 的示例。

