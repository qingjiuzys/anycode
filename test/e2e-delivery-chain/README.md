# E2E Delivery Chain

从 0 搭建 anyCode 交付链端到端测试：办公（xlsx/docx/pptx）、编码（mock + live）、HTML 创作。

## 前置

- `cargo build --release -p anycode`
- `python3` + `openpyxl`, `python-docx`, `python-pptx`
- 可选 PDF 引擎（anycode-pdf）
- `~/.anycode/config.json` 含可用 LLM（live 场景）
- Dashboard @ `http://127.0.0.1:43180`

## 一键执行

```bash
cd test/e2e-delivery-chain
chmod +x reset.sh bootstrap.sh
node run_all.mjs
```

跳过 reset、仅校验并生成报告：

```bash
SKIP_RESET=1 VERIFY_ONLY=1 node run_all.mjs
```

严格质量门禁（WARN 视为失败）：

```bash
E2E_STRICT_QUALITY=1 node run_all.mjs
```

## 质量体系

- 用户原始 `prompt.md` **不改写**；运行时由 `build_brief.mjs` 注入 delivery contract（见 `out/<scenario>.brief.md`）。
- 验收标准见 `shared/quality/rubric.json` 与 `artifact_contracts.json`。
- `office_verify.py --json` 输出 0–100 分、等级（PASS/WARN/FAIL）、扣分项。

## 目录

- `reset.sh` — 备份并清空 projects.db / tasks / dashboard（保留 config）
- `bootstrap.sh` — 工作区 `~/.anycode/workspace/e2e-delivery`、注册项目、装 skills
- `scenarios/01..07` — 各场景 `prompt.md` + `run.mjs` + `verify.mjs`
- `out/REPORT-<date>.md` — 汇总报告

## 单场景

```bash
node scenarios/04-coding-mock/run.mjs
node scenarios/04-coding-mock/verify.mjs
```

## 08 复杂交付冲刺 v2（推荐单独跑）

多步链：**12 页 PPT + 双 Excel + Word + HTML 看板 + QA 清单 + Rust 4 bug 修复 + CHANGELOG + manifest**。

```bash
cd test/e2e-delivery-chain
SKIP_RESET=1 bash bootstrap.sh
COMPLEX_ONLY=1 node run_all.mjs
node scenarios/08-complex-delivery/verify.mjs
node generate_complex_audit.mjs
```

Harness 会自动生成 `out/e2e-anycode.config.json`（turns=9999 + security bypass）。

严格模式：`E2E_STRICT_QUALITY=1`（WARN/P1 视为失败）。仅验证：`VERIFY_ONLY=1 COMPLEX_ONLY=1`。

Checkpoint：`out/08-checkpoints/{code,office,integration}.json`。

v3 设计：[`shared/quality/COMPLEX_V3.md`](shared/quality/COMPLEX_V3.md)
