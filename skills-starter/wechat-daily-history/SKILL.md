---
name: wechat-daily-history
description: Auto-configure local WeChat history access and query daily chat records as Markdown tables.
description_zh: 自动配置本机微信聊天记录数据源，按天查询并输出 Markdown 分析表格。
category: business
---

# wechat-daily-history

> **中文**：一键从本机微信加密库提取密钥并查询聊天记录（**非 iLink 扫码机器人通道**）。  
> **English**: One-shot local WeChat DB key extraction + daily history queries (not iLink bot QR bind).

## 重要说明

- **iLink 微信扫码**（`anycode channel wechat`）= 机器人收发消息，**不能**读 Mac 本地加密聊天库。
- **本 skill** = 从已登录微信进程内存提取 SQLCipher 密钥，读取 `db_storage`（**查询时先复制 DB/WAL/SHM 到临时快照，不直接打开 live 库**）。
- macOS 首次提取可能需 **临时关闭 SIP**（`csrutil disable`），见 `doctor wechat-history`。

## 一键配置（推荐）

```bash
anycode wechat history setup
```

或：

```bash
scripts/wechat-history-setup.sh setup
```

等价于 `install`（brew sqlcipher/llvm + vendor 工具）+ `ensure`（内存扫密钥 + 写 config）。

**你需要：**

1. Mac 微信已登录且 **WeChat 进程在运行**
2. 终端提示时允许 attach（部分环境需 sudo）
3. 若报 `sip_blocks_memory_scan`：进恢复模式临时关 SIP 后重跑

## 工作流（agent）

### 1. 自动配置

```
Skill { "name": "wechat-daily-history", "args": ["setup"] }
```

或分步：`install` → `ensure` / `extract-keys`

子命令：

| args | 作用 |
| --- | --- |
| `setup` | 一键 install + ensure |
| `install` | brew + clone wechat-db-decrypt vendor |
| `ensure` | 写 config；缺密钥则 extract-keys |
| `extract-keys` | 内存扫描 → `~/.anycode/wechat-history/wechat_keys.json` |
| `status` | 只探测 JSON |

诊断：

```bash
anycode doctor wechat-history
```

### 2. 查询

```
QueryWeChatHistory {
  date: "YYYY-MM-DD",
  format: "markdown_table"
}
```

### 3. 分析输出表格

| 时间 | 会话 | 发送者 | 类型 | 摘要 | 标签 |

## 常见 error

| error | 处理 |
| --- | --- |
| `wechat_not_running` | 打开并登录 Mac 微信 |
| `sip_blocks_memory_scan` | 临时关 SIP 后重跑 `setup` |
| `wechat_keys_extract_failed` | 查看 `~/.anycode/wechat-history/extract-keys.log` |
| `keys_extracted_but_db_verify_failed` | 重启微信后重跑 `extract-keys` |
| `wechat_db_not_found` | 确认微信已在本机登录过 |

## 规则

- 只读；禁止发微信消息、禁止改微信数据库；查询走临时快照副本，不直接 attach live DB 文件。
- 文件消息会输出 `attachments[]`（发送者、文件名、解析状态）；Excel 解析需 `parse_files: true`。
- 密钥文件权限 `600`，勿提交 git。
- 配置失败时不要伪造聊天记录。

## 示例

```
请用 wechat-daily-history：先 setup 一键配置，再查今天微信聊天记录，输出 Markdown 表格。
```
