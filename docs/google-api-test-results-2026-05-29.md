# Google API 功能验证结果（2026-05-29）

配置：`provider=google`，`base_url=https://generativelanguage.googleapis.com/v1beta/openai/chat/completions`，测试用配置 `/tmp/anycode-google-test.json`（勿提交）。

## 结果汇总

| Layer | Case | Model | Result | Notes |
|-------|------|-------|--------|-------|
| L0 | curl OpenAI-compat | flash | **pass** | HTTP 200 |
| L0 | doctor all | home config | **pass** | `llm.api_key` ok；`llm.google_fallback` warn（预期） |
| L1 | GET model-catalog | — | **pass** | `google_models` 含 flash/pro |
| L1 | POST /api/settings/llm chat probe | flash | **pass** | `chat ok: Pong`，~4.7s |
| L1 | PUT models + POST .../test | flash | **pass** | saved + draft 探针均 ok |
| L1 | fixture_api | — | **pass** | `cargo test -p anycode-dashboard fixture_api` |
| L2 | Dashboard UI model 页 | — | **pass** | catalog/test/i18n；UI 显示 `chat ok: Pong` |
| L2 | Conversations 页 | — | **pass** | 列表与 transcript 区域正常加载 |
| L3 | 单轮 line REPL | flash | **pass** | 返回 Rust 一句话介绍 |
| L3 | 三轮 line REPL | flash | **pass** | 2→6→再见，历史连贯 |
| L4 | T4-1 explore | flash | **partial** | 2 轮工具（Bash+FileRead）；FileRead 报 not found（非 sandbox 时相对路径未解析到 `-C` 目录） |
| L4 | T4-2 bugfix | flash | **fail** | FileRead 失败；未改代码 |
| L4 | T4-2 retry（绝对路径提示） | flash | **fail** | `timeout` 连接 Google 失败（10 次重试） |
| L4 | T4-2 retry（sandbox） | flash | **fail** | 同上 timeout |
| L4 | T4-3 pro 复杂 | pro | **skip** | 未执行（L4 阻塞） |
| L5 | goal + done-when | pro | **skip** | 未执行（L4 阻塞） |

## 阻塞与发现

1. **网络 timeout**：部分时段 `generativelanguage.googleapis.com` 连接超时（`API_TIMEOUT_MS=180000`），导致 `anycode run` 在 turn 1 失败；同期 curl/探针曾成功，属间歇性。
2. **FileRead + 非 sandbox**：`-C` 临时目录下 `src/lib.rs` 存在，但 `FileRead` 在 `security.sandbox_mode=false` 时用进程 cwd 解析相对路径，导致 “File not found”。建议在 `run -C <dir>` 场景默认对 FileRead/Edit 解析到 working_directory，或文档要求开启 sandbox。
3. **安全**：API Key 已在对话中暴露，请在 [Google AI Studio](https://aistudio.google.com/apikey) **轮换**。

## 恢复

测试曾将 `/tmp/anycode-google-test.json` 同步到 `~/.anycode/config.json`；备份：`~/.anycode/config.json.bak-google-test`。

```bash
cp ~/.anycode/config.json.bak-google-test ~/.anycode/config.json   # 如需还原
```

## 截图

- `google-test-model-settings-zh.png`（模型设置页，测试成功）
- `google-test-model-settings-en.png`（英文 i18n）

保存在 Cursor 临时截图目录。

## 重试（2026-05-29 下午）

| 检查 | 结果 |
|------|------|
| `curl generativelanguage...` ×3（60s） | **fail** — Connection timed out |
| `curl https://www.google.com` | **fail** — timeout |
| `curl https://www.baidu.com` | **pass** — HTTP 200 |
| DNS `generativelanguage.googleapis.com` | 可解析（142.250.x / 142.251.x） |
| 本机常见代理端口 7890/10808 等 | 未监听 |
| Dashboard `POST /api/settings/llm` | **fail** — 20s 无响应（探针同样打 Google） |

**结论（第一次重试）**：当时到 Google 的 TCP 不通（非 Key 无效）。

## 再次重试（2026-05-30）

| 检查 | 结果 |
|------|------|
| `curl generativelanguage...` | **可达** ~1.3s，但 HTTP **400** |
| 错误正文 | `User location is not supported for the API use.` / `FAILED_PRECONDITION` |
| Dashboard `POST /api/settings/llm` | **fail** — 同上 geo 错误 |
| `anycode run` T4-2 bugfix | **fail** — turn 1 LLM geo 400 |
| line REPL | **fail** — 同上 geo |

**结论（第二次重试）**：网络已恢复，但 Google **按地区拒绝**（与 doctor 的 `llm.google_fallback` warn 一致）。仅 Google、无 fallback 时，L4/L5 无法在当前 IP/地区完成。

**可行出路（任选其一）**：
1. VPN 到 Google AI Studio 支持的地区（美/日等），再重跑探针与 `anycode run`
2. 在 `~/.anycode/config.json` 配置 `runtime.model_fallback`（第二厂商），geo 时自动切换

## 第三次重试（2026-05-30 09:49）— 成功

前提：`security.sandbox_mode: true`（修复 FileRead 相对路径）。

| 项目 | Model | 结果 | 详情 |
|------|-------|------|------|
| curl 探针 | flash | **pass** | HTTP 200 |
| Dashboard probe | flash | **pass** | `chat ok:` |
| **T4-2 bugfix** | flash | **pass** | 4 轮：FileRead → Edit → Bash `cargo test` → 完成；测试 1 passed |
| **L5 goal** | pro | **fail** | 429 免费配额用尽（`gemini-2.5-pro` limit: 0） |
| **L5 goal** | flash | **pass** | `attempts=1 completed=true`；自主调查+修复+`cargo test` 通过 |

**结论**：Google API + Agent 自主工具链验证通过。复杂任务建议用 `gemini-2.5-flash`（当前 Key 的 pro 免费层配额为 0）；生产环境建议配置 billing 或 `runtime.model_fallback`。

## 免费模型专项（2026-05-30）

已切换配置：`gemini-2.5-flash`（`/tmp/anycode-google-free.json` + `~/.anycode/config.json`）。

| 模型 | 免费层探测 | 说明 |
|------|-----------|------|
| `gemini-2.5-flash` | **可用**（RPM 20/分钟） | 冷却后 curl/Agent 均成功；连续探测易 429 |
| `gemini-2.0-flash` | **429 limit:0** | 该 Key 当日/当前免费配额已用尽 |
| `gemini-2.5-pro` | **429 limit:0** | 免费层不可用，需 billing |
| `gemini-1.5-flash` | **404** | OpenAI-compat 端点不支持此 id |

**建议**：日常用 `gemini-2.5-flash` + `security.sandbox_mode: true`；避免短时间多次探针；429 时等待 ~60s 再试。

恢复连通后建议命令：

```bash
export HTTP_PROXY=http://127.0.0.1:<你的代理端口>   # 如有
export HTTPS_PROXY=$HTTP_PROXY

ANYCODE=target/release/anycode
TESTDIR=/path/to/bugfix-copy   # 或重新 mktemp + cp scripts/eval/fixtures/bugfix-repo

# 建议开启 sandbox，避免 FileRead 相对路径问题
# 在 config 中 security.sandbox_mode: true

$ANYCODE --config /tmp/anycode-google-test.json -C "$TESTDIR" -I run \
  --agent general-purpose "读取 src/lib.rs，把 add 改成 a+b，运行 cargo test"

$ANYCODE --config /tmp/anycode-google-test-pro.json -C "$TESTDIR" -I run \
  --goal "让 cargo test 全部通过" --done-when "cargo test" --max-goal-attempts 5 \
  "调查并修复测试失败"
```
