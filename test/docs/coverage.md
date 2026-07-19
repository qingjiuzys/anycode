# 代码覆盖口径

## 发布门禁（非全仓 95% 行覆盖）

- **风险覆盖**：`requirements/catalog.toml` 中 P0/P1 要求 ≥95% 有至少一个通过的自动化 case。
- **模块覆盖**（增量防回退，首版仅建基线）：
  - P0 安全/认证/计费/审计：`line` + `branch` ≥90%
  - 普通核心模块：≥85%
  - 全仓不得较上一基线下降超过 2 个百分点

## 采集方式

```bash
./scripts/test-coverage.sh test/results/<run-id>/coverage
```

| 范围 | 工具 | 输出 |
|------|------|------|
| Rust workspace + `account-service` | `cargo llvm-cov` | `coverage/rust.lcov` |
| Dashboard UI | Vitest V8 | `coverage/dashboard-ui/` |

## CI 对齐

- 常规 PR：`.github/workflows/ci.yml`（无覆盖率，快速）
- 评测/大版本：`.github/workflows/eval.yml`（`--coverage` 可选）
- 本地提交前：与 CI 一致跑 `cargo fmt/clippy/test`，覆盖率由 `release-candidate` profile 触发

## 首次基线

首次运行只记录真实覆盖率到 `test/results/`，审核后可将摘要提交到 `test/baselines/*.json`（见 `schema.json`）。

未覆盖的 P0/P1 要求在 `catalog.toml` 中标注 `owner`、`reason`、`due_version`。
