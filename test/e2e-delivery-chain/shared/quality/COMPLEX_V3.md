# Complex Delivery v3 设计草案

> 激活：`COMPLEX_LEVEL=v3` + `RUN_COMPLEX=1`（默认关闭，nightly CI）

## 相对 v2 的增量

| 维度 | v2 | v3 |
| --- | --- | --- |
| 数据 | 9 行 3 日 CSV | **30 天 × 多渠道 CSV** |
| 冲突 | 无 | CSV vs MD **故意 1% 偏差** + `reconciliation_note.md` |
| 仓库 | 1 个 Rust workspace | **+ e2e-metrics-api**（只读 HTTP JSON） |
| HTML | 静态 KPI | 必须引用 `metrics_snapshot.json` 或 API |
| PPT | ≥12 页 / 8 主题 | **≥15 页 / 10 主题** |
| Git | 1+ commit | **至少 2 commit**：`fix(...)` + `docs(changelog)` |

## 目录规划

```
shared/fixtures/
  sales_june_30d.csv
  sales_executive_summary.md   # 故意与 CSV 差 1%
  e2e-complex-repo/            # 现有 Rust workspace
  e2e-metrics-api/             # 新建：axum/actix 只读 summary API

scenarios/09-complex-delivery-v3/
  prompt.md
  run.mjs
  verify.mjs
```

## 验证器要点

- `reconciliation_note.md` 必须解释 1% 偏差来源与采用哪份为准
- API smoke：`curl localhost:PORT/api/v1/metrics/summary` 与 manifest 一致
- HTML 内嵌或 fetch `metrics_snapshot.json`
- 禁止手算：要求 `artifacts/aggregate_script.py` 或等效脚本存在且输出与 CSV 一致

## 实现顺序

1. 生成 30d fixture + conflict MD
2.  scaffold `e2e-metrics-api` + harness bootstrap 启动
3. `complex_v3_verify.py` 或扩展 `complex_verify.py`
4. 场景 09 prompt + contract
5. nightly CI job

契约源文件：[`complex_v3.json`](complex_v3.json)
