# 评分规则

## 状态定义

| 状态 | 含义 |
|------|------|
| `passed` | 确定性断言全部满足 |
| `failed` | 断言失败、超时、或非预期退出码 |
| `skipped` | 显式跳过（如无 `--models` 的 live LLM case） |
| N/A（metrics） | 模型能力不适用（如 1B `tools=false` 的 browser/skills） |

## 优先级

1. **确定性验证**：编译、单测、文件结构、API JSON shape、DOM 状态
2. **静态分析**：Semgrep/Bandit/Ruff/Clippy（security profile，后续场景）
3. **固定 rubric judge**：独立模型、盲评；不得与被测模型相同

## Fail-fast

- 预置 fixture 缺失 → **失败**，禁止 `test.skip` 掩盖 P0 API 路径
- E2E 服务器必须设置 `ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1`（仅 loopback）
- 模型错误**不重试**；基础设施（dashboard 未启动）可在同 run 内重试一次

## 采集指标

- 成功率、pass@1、稳定通过率（关键样本 ≥3 次重复，full profile）
- 首 token / 总耗时、输入输出 token、人民币成本
- tool 调用序列、无效重试、用户纠正次数
- 文件 diff、产物路径、错误类别（来自 trace/replay/usage）

## 大版本门禁

- P0 case：**100%** 通过
- P0/P1 风险自动化覆盖：**≥95%**
- 安全高危：**0**（SARIF 阻断项）
- 行业 benchmark：首版仅建基线；显著 pass rate 下降或成本超阈值才阻断
