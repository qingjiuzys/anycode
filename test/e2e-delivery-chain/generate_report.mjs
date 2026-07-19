#!/usr/bin/env node
/** Generate quality-focused REPORT-<date>.md from out/*.json */
import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { auditScenarioRun } from "./shared/utils/process_audit.mjs";

const harnessRoot = dirname(fileURLToPath(import.meta.url));
const outDir = join(harnessRoot, "out");
const date = new Date().toISOString().slice(0, 10);

const SCENARIOS = [
  { id: "01-office-xlsx", suite: "office", title: "六月销售 CSV → Excel", content: true },
  { id: "02-office-docx", suite: "office", title: "销售日报 Word", content: true },
  { id: "03-office-pptx", suite: "office", title: "3 页汇报 PPT", content: true },
  { id: "04-coding-mock", suite: "coding", title: "Mock eval", content: false },
  { id: "05-coding-live", suite: "coding", title: "Live bugfix", content: false },
  { id: "06-html-page", suite: "html", title: "landing.html", content: true },
  { id: "07-html-md-skill", suite: "html", title: "md-to-pdf skill", content: true },
  { id: "08-complex-delivery", suite: "complex", title: "复杂交付冲刺（办公+编码+git+manifest）", content: true },
];

function readJson(name) {
  const p = join(outDir, name);
  if (!existsSync(p)) return null;
  try {
    return JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

function deliveryVerdict(score, grade, degraded) {
  if (grade === "PASS" && !degraded) return "可直接交付";
  if (grade === "PASS" && degraded) return "可交付（有降级，需标注）";
  if (grade === "WARN" && score >= 85) return "可交付（有轻微警告）";
  if (grade === "WARN") return "需人工修订";
  return "不可交付";
}

function displayGrade(grade, score) {
  if (grade === "WARN" && score >= 85) return "PASS_WITH_WARNINGS";
  return grade;
}

function readVerify(scenarioId) {
  const num = scenarioId.split("-")[0];
  const name = num === "08" ? "08-verify.json" : `${num}-verify.json`;
  return readJson(name);
}

const results = SCENARIOS.map((sc) => {
  const run = readJson(`${sc.id}.json`);
  const verify = sc.content ? readVerify(sc.id) : null;
  const procAudit = run?.processAudit ?? (run ? auditScenarioRun(run) : {});
  const degraded = procAudit.degraded || verify?.evidence?.process?.degraded || verify?.evidence?.audit?.degraded;
  const score = verify?.score ?? (sc.content ? 0 : run?.pass ? 100 : 0);
  const grade = verify?.grade ?? (run?.pass ? "PASS" : "FAIL");
  const display = sc.content ? displayGrade(grade, score) : grade;
  const pass =
    sc.content
      ? grade === "PASS" || (grade === "WARN" && score >= 70 && process.env.E2E_STRICT_QUALITY !== "1")
      : run?.pass === true;
  return {
    ...sc,
    run,
    verify,
    process: procAudit,
    degraded,
    score,
    grade,
    display,
    pass,
    verdict: sc.content ? deliveryVerdict(score, grade, degraded) : run?.pass ? "链路可用" : "失败",
  };
});

const contentResults = results.filter((r) => r.content);
const scores = contentResults.map((r) => r.score).filter((s) => s > 0);
const avgScore = scores.length ? Math.round(scores.reduce((a, b) => a + b, 0) / scores.length) : 0;
const minScore = scores.length ? Math.min(...scores) : 0;

const lines = [];
lines.push("# anyCode 交付链 E2E 质量报告");
lines.push("");
lines.push(`生成时间：${new Date().toISOString()}`);
lines.push("");
lines.push("## 1. 质量评分总览");
lines.push(`- **内容类均分**：${avgScore}/100`);
lines.push(`- **最低分**：${minScore}/100`);
lines.push(`- **执行通过**：${results.filter((r) => r.pass).length}/${results.length}`);
lines.push(`- **降级项**：${results.filter((r) => r.degraded).map((r) => r.id).join(", ") || "无"}`);
lines.push(`- **业务交付建议**：${avgScore >= 85 && minScore >= 70 ? "整体可进入业务验收" : "建议先修复 WARN/FAIL 项再交付"}`);
lines.push("");
lines.push("| 场景 | 质量分 | 等级 | 降级 | 交付结论 |");
lines.push("| --- | ---: | --- | --- | --- |");
for (const r of results) {
  const scoreCell = r.content ? String(r.score) : "—";
  lines.push(`| ${r.id} | ${scoreCell} | ${r.display} | ${r.degraded ? "是" : "否"} | ${r.verdict} |`);
}
lines.push("");
lines.push("## 2. 分项质量审计");
for (const r of results) {
  lines.push(`### ${r.id} — ${r.title}`);
  lines.push(`- **等级**：${r.display} | **质量分**：${r.content ? r.score : "N/A"} | **结论**：${r.verdict}`);
  if (r.run?.briefPath) lines.push(`- **增强 brief**：\`${r.run.briefPath}\`（用户原 prompt 未改写）`);
  if (r.verify?.artifact) lines.push(`- **产物**：\`${r.verify.artifact}\``);
  if (r.run?.processAuditPath) lines.push(`- **过程审计**：\`${r.run.processAuditPath}\``);
  if (r.degraded) lines.push(`- **过程**：Skill 降级后完成（DEGRADED）`);
  if (r.verify?.dimensions) {
    const d = r.verify.dimensions;
    lines.push(`- **维度**：数据 ${d.data_accuracy ?? "—"} / 完整 ${d.task_completeness ?? "—"} / 表达 ${d.business_expression ?? "—"} / 呈现 ${d.presentation ?? "—"} / 过程 ${d.process_trust ?? "—"}`);
  }
  if (r.verify?.findings?.length) {
    lines.push("- **扣分项**：");
    for (const f of r.verify.findings) {
      lines.push(`  - [${f.severity}] ${f.message}`);
    }
  } else if (r.run?.pass && !r.content) {
    lines.push("- **扣分项**：无（编码链路）");
  }
  lines.push("");
}
lines.push("## 3. 风险与缺口");
lines.push("- 质量 contract 由系统注入，不依赖用户 prompt 专业度。");
lines.push("- WARN 表示可读但不建议直接对外交付；`E2E_STRICT_QUALITY=1` 时 WARN 视为失败。");
lines.push("- 07 若缺 `report_table.audit.json` 或 Skill 静默 fallback，过程可信度扣分。");
lines.push("- 08 复杂冲刺：办公三件套 + Rust workspace 修 bug + 本地 git commit + `DELIVERY_MANIFEST.json`；默认 `run_all` 不跑，需 `COMPLEX_ONLY=1` 或 `RUN_COMPLEX=1`。");
lines.push("");
lines.push("## 附录：环境与证据");
const anycode = join(harnessRoot, "../../target/release/anycode");
const version = existsSync(anycode) ? (spawnSync(anycode, ["--version"], { encoding: "utf8" }).stdout || "").trim() : "unknown";
lines.push(`- anycode：\`${version}\``);
lines.push(`- 工作区：\`${existsSync(join(outDir, "workspace_path.txt")) ? readFileSync(join(outDir, "workspace_path.txt"), "utf8").trim() : "n/a"}\``);
const backupDir = join(process.env.HOME, ".anycode/backups");
const backups = existsSync(backupDir) ? readdirSync(backupDir).filter((f) => f.startsWith("pre-e2e-")).sort() : [];
lines.push(`- DB 备份：\`${backups.length ? join(backupDir, backups[backups.length - 1]) : "n/a"}\``);

const reportPath = join(outDir, `REPORT-${date}.md`);
writeFileSync(reportPath, lines.join("\n") + "\n");
console.log(reportPath);
