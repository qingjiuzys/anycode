# anyCode 持续测试基线

本目录是 anyCode 大版本评测与风险回归的统一入口，不替代现有 `cargo test` / Playwright CI。

## 快速开始

```bash
# 1. 环境检查
python3 test/run.py doctor

# 2. 日常 smoke（≤20 分钟，无真实 LLM）
python3 test/run.py --profile smoke

# 3. 场景语料 fixture/trajectory 门禁（CI，无真实 LLM）
python3 test/run.py --profile fixture-ci

# 4. 多轮 Agent 场景评测（必须显式选择真实模型；默认重复 3 次）
python3 test/run.py --profile live-model --models local-1b,agnes

# 5. 大版本完整评测（8–12 小时）
python3 test/run.py --profile full --models local-1b,agnes,cloud-auto
```

可选依赖：

```bash
python3 -m venv test/.venv && source test/.venv/bin/activate
pip install -r test/requirements.txt
```

## 报告位置

每次运行写入 `test/results/<run-id>/`：

| 文件 | 说明 |
|------|------|
| `summary.md` | 人类可读摘要 |
| `cases.jsonl` | 逐例结果 |
| `junit.xml` | CI 集成 |
| `traces/<case>.json` | 多轮 prompt、断言结果与 trajectory gate 明细 |
| `coverage/` | 覆盖率快照（若启用） |
| `models/<model>/report.md` | 按模型优化建议 |

## 目录

| 路径 | 用途 |
|------|------|
| [docs/](docs/) | 覆盖口径、评分、模型矩阵 |
| [manifests/](manifests/) | smoke / release-candidate / full 套件 |
| [requirements/catalog.toml](requirements/catalog.toml) | P0/P1 风险要求映射 |
| [cases/](cases/) | 场景用例定义 |
| [fixtures/](fixtures/) | 只读 fixture（浏览器站点、多文件项目等） |
| [benchmarks/](benchmarks/) | 行业标准基准适配器 |
| [baselines/](baselines/) | 已审核的基线摘要 JSON |

细节见 [docs/overview.md](docs/overview.md)。
