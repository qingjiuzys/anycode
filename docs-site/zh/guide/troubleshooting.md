---
title: 排错
description: 按“现象”快速定位 anyCode 常见问题，并给出下一步操作。
summary: 先解决命令、setup、扫码、API 报错，再进入进阶诊断。
read_when:
  - setup、channel 绑定或命令执行失败。
  - 需要一个快速可执行的排错清单。
---

# 排错

适合出现报错后，希望先快速恢复可用的用户。

使用方式：

1. 先按“现象”找到最接近的一节
2. 按顺序执行该节里的检查项
3. 快速检查无效时，再看“进阶诊断”

## 1）`anycode` 命令找不到

1. 先执行 `anycode --help`
2. 如果提示找不到命令，按安装脚本输出修正 PATH
3. 新开一个终端再试
4. 仍失败就按 [安装](./install) 重装

## 2）`setup` 不能交互 / 卡住

- 请在本机真实终端运行（不要在 CI / 无头环境中交互）
- 如果你只想先配某个 channel，可直接指定：

```bash
anycode setup --channel wechat
anycode setup --channel telegram
anycode setup --channel discord
```

预期输出：setup 会直接进入你指定的 channel 流程。

## 3）微信扫码失败

- 在有图形界面/浏览器的机器执行：

```bash
anycode channel wechat
```

预期输出：出现扫码绑定流程并提示后续确认步骤。

- 如果任务跑到错误目录，在微信里用 `/cwd` 切到项目目录。

## 4）API 调用报错

- 重新执行 `setup`，确认 provider / model / api key
- 确认 endpoint 与 provider 协议匹配（OpenAI 兼容接口 vs 厂商原生接口）
- 使用 Google provider 时，优先用 setup 自动填充的默认 endpoint

## 5）审批提示影响使用

- `require_approval=true` 时，敏感工具会要求确认
- 如果你明确理解风险，且仅本次跳过：

```bash
anycode run --ignore-approval --agent general-purpose "..."
```

预期输出：本次进程执行任务时不再弹审批确认。

## 进阶诊断（可选）

- **MCP / `McpAuth` / OAuth（无 GUI）**：anycode 不会替你弹浏览器。用动态 **`mcp__…__authenticate`** 或 **`McpAuth`**，看 MCP 子进程 **stderr**（与 CLI 同一终端），再在系统浏览器里完成授权。详见 [配置与安全 — MCP OAuth](./config-security) 与环境变量 **`ANYCODE_MCP_READ_TIMEOUT_SECS`** / **`ANYCODE_MCP_CALL_TIMEOUT_SECS`**（调用挂起时）。
- 开发者日志与测试：看 [开发与贡献](./development)

## 仍然无法解决

- 提 Issue 时请附：
  - 系统版本
  - `anycode --version`
  - 脱敏后的 `~/.anycode/config.json`（去掉 API Key）
