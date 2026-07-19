# 模型矩阵

## 必测模型

| 别名 | Registry / API ID | 说明 |
|------|-------------------|------|
| `local-1b` | `sglang-minicpm5-1b` | SGLang + `--tool-call-parser minicpm5`（官方推荐）；`LOCAL_1B_BACKEND=ollama` 可回退 |
| `managed-minicpm5-1b` | `managed-minicpm5-1b` | Desktop 托管 GGUF（独立路径） |
| `agnes` | `agnes-chat` | 完整 agent + tool 矩阵 |
| `cloud-auto` | `cloud-auto` | 云路由回归对照 |

探测入口：`python3 test/run.py --profile full --models local-1b,agnes,cloud-auto`

## local-1b 后端

默认 **SGLang**（`test/scripts/ensure_sglang_minicpm5.sh`）：

```bash
python -m sglang.launch_server \
  --model-path openbmb/MiniCPM5-1B \
  --tool-call-parser minicpm5 \
  --context-length 32768 \
  --port 30000
```

- 需 **NVIDIA GPU**；无 GPU 时可设 `SGLANG_BASE_URL=http://<gpu-host>:30000/v1/chat/completions`
- 回退 Ollama：`LOCAL_1B_BACKEND=ollama`（历史 GGUF 路径，多轮需文本协议压平）

## 能力探测

通过 Dashboard API（`test/runner/model_probe.py`）：

1. `GET /api/local-models` — 本地 manifest、`capabilities.tools`
2. `GET /api/settings/models` — registry 配置
3. `GET /api/cloud/session` — 云链接状态
4. 可选 `POST /api/settings/models/{id}/test` — 连通性
5. **`sglang-minicpm5-1b` / `managed-minicpm5-1b` 且 runtime 就绪** — 执行只读 Glob 工具任务，验证 trace 含 `tool_call_start` → `tool_call_end`

## tools 不可用处理

`sglang-minicpm5-1b` 或 `managed-minicpm5-1b` 若 metadata 或实时 tool probe 表明 **tools 不可用**：

- `browser`、`skills` 套件标记 **N/A**
- 报告 `models/local-1b/report.md` 中列出产品差距
- 纯文本/代码/长上下文得分仍单独呈现

## 硬件记录

完整基线 JSON（`test/baselines/schema.json`）应包含：

- OS / CPU / RAM / GPU
- 本地模型 GGUF SHA256、量化档（Ollama 路径）或 SGLang 部署参数
- Agnes 服务版本
- 探测时延与内存峰值（后续 scenario 填充）

## 不可比任务

不把 browser/skills 失败算入 1B 总分；Agnes 与 Cloud Auto 展示完整 agent 成功率与成本。
