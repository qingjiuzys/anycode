#!/usr/bin/env node
/**
 * Full complex-delivery audit report + optimization plan (post-run).
 */
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { auditScenarioRun, auditFromTaskLog, extractTaskLogPath } from "./shared/utils/process_audit.mjs";
import {
  readComplexCheckpoint,
  snapshotWorkspaceStageSync,
  inferFailedStages,
  checkpointDir,
} from "./shared/utils/complex_checkpoints.mjs";

const harnessRoot = dirname(fileURLToPath(import.meta.url));
const outDir = join(harnessRoot, "out");
const date = new Date().toISOString().slice(0, 10);

function readJson(name) {
  const p = join(outDir, name);
  if (!existsSync(p)) return null;
  try {
    return JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

function readText(name) {
  const p = join(outDir, name);
  return existsSync(p) ? readFileSync(p, "utf8").trim() : null;
}

function listArtifacts(workspace) {
  const pack = join(workspace, "artifacts/executive_pack");
  const items = [];
  if (existsSync(pack)) {
    for (const f of readdirSync(pack)) {
      const p = join(pack, f);
      if (statSync(p).isFile()) items.push({ path: `artifacts/executive_pack/${f}`, bytes: statSync(p).size });
    }
  }
  for (const rel of ["artifacts/executive_dashboard.html", "artifacts/DELIVERY_MANIFEST.json"]) {
    const p = join(workspace, rel);
    if (existsSync(p)) items.push({ path: rel, bytes: statSync(p).size });
  }
  return items;
}

function extractTurnPeak(run) {
  const text = `${run?.stdoutTail ?? ""}\n${run?.stderrTail ?? ""}`;
  const turns = [...text.matchAll(/turn=(\d+)/g)].map((m) => Number(m[1]));
  return turns.length ? Math.max(...turns) : null;
}

function resolveProcessAudit(run) {
  if (!run) return { toolChain: [], errors: [], degraded: false, matchedLog: null };
  let audit = auditScenarioRun(run);
  const combined = `${run.stdoutTail ?? ""}\n${run.stderrTail ?? ""}`;
  const logPath = run.processAudit?.matchedLog ?? extractTaskLogPath(combined);
  if (logPath && existsSync(logPath)) {
    audit = auditFromTaskLog(logPath);
  }
  if (!audit.toolChain?.length && combined) {
    audit = auditScenarioRun({ ...run, stdoutTail: combined, stderrTail: "" });
  }
  return audit;
}

function readRunTiming(logPath) {
  if (!logPath || !existsSync(logPath)) return null;
  const text = readFileSync(logPath, "utf8");
  const start = text.match(/\[task_start\][^\n]*/);
  const end = text.match(/\[task_end\][^\n]*/);
  return { start: start?.[0] ?? null, end: end?.[0] ?? null, bytes: text.length };
}

const verify = readJson("08-verify.json");
let run = readJson("08-complex-delivery.json");
const v2 = JSON.parse(readFileSync(join(harnessRoot, "shared/quality/complex_v2.json"), "utf8"));
const workspace = readText("workspace_path.txt") ?? verify?.artifact ?? "";
const artifacts = workspace ? listArtifacts(workspace) : [];
const process = resolveProcessAudit(run);
if (run && process.toolChain?.length) {
  run = { ...run, processAudit: process };
  writeFileSync(join(outDir, "08-complex-delivery.json"), JSON.stringify(run, null, 2) + "\n");
}

const snapshot = workspace ? snapshotWorkspaceStageSync(workspace, spawnSync) : null;
const failedStages = snapshot ? inferFailedStages(snapshot) : [];
const turnPeak = extractTurnPeak(run);
const timing = readRunTiming(process.matchedLog);

const lines = [];
lines.push("# 复杂交付冲刺（08）完整审计报告");
lines.push("");
lines.push(`生成时间：${new Date().toISOString()}`);
lines.push(`场景版本：**complex_v2**（PPT ≥${v2.min_ppt_slides} 页 · ≥${v2.min_ppt_themes} 主题 · 7 类产物）`);
lines.push("");

lines.push("## 1. 执行摘要");
if (verify) {
  lines.push("| 指标 | 值 |");
  lines.push("| --- | --- |");
  lines.push(`| 质量分 | **${verify.score}/100** |`);
  lines.push(`| 等级 | **${verify.grade}**（pass=${verify.pass}） |`);
  lines.push(`| PPT 页数 | ${verify.evidence?.pptx_slides ?? verify.evidence?.pptx?.slides ?? "—"} |`);
  lines.push(`| PPT 主题 | ${(verify.evidence?.pptx?.ppt_themes_matched ?? []).join(", ") || "—"} |`);
  lines.push(`| cargo test | ${verify.evidence?.cargo_exit === 0 ? "通过" : "失败"} |`);
  lines.push(`| Git commits | ${(verify.evidence?.git_commits ?? []).length || "—"} |`);
  lines.push(`| Turn 峰值 | ${turnPeak ?? "—"} |`);
  lines.push(`| 失败阶段 | ${failedStages.length ? failedStages.join(", ") : "无"} |`);
  lines.push(`| 会话 | ${run?.sessionId ?? "—"} |`);
} else {
  lines.push("*尚未运行 verify*");
}
lines.push("");

lines.push("## 2. 产物清单");
if (artifacts.length) {
  lines.push("| 文件 | 大小 |");
  lines.push("| --- | ---: |");
  for (const a of artifacts) lines.push(`| \`${a.path}\` | ${a.bytes} B |`);
} else {
  lines.push("*工作区无产物*");
}
lines.push("");

lines.push("## 3. 语义验证证据");
if (verify?.evidence) {
  const e = verify.evidence;
  if (e.docx?.docx_sections_matched) lines.push(`- Word 章节：${e.docx.docx_sections_matched.join("、")}`);
  if (e.pptx?.ppt_themes_matched) lines.push(`- PPT 主题：${e.pptx.ppt_themes_matched.join("、")}`);
  if (e.qa_signoff?.qa_missing?.length) lines.push(`- QA 缺失 token：${e.qa_signoff.qa_missing.join("、")}`);
  else if (e.qa_signoff) lines.push("- QA：全部 required token 命中");
}
lines.push("");

lines.push("## 4. 扣分项");
if (verify?.findings?.length) {
  for (const f of verify.findings) lines.push(`- **[${f.severity}]** ${f.message}`);
} else {
  lines.push("无扣分项。");
}
lines.push("");

lines.push("## 5. 过程审计");
lines.push(`- 工具链（${process.toolChain?.length ?? 0} 次）：${(process.toolChain ?? []).slice(0, 24).join(" → ")}${(process.toolChain?.length ?? 0) > 24 ? " …" : ""}`);
lines.push(`- Skill 降级：${process.degraded ? "是" : "否"}`);
if (process.errors?.length) {
  lines.push("- 工具错误：");
  for (const e of process.errors.slice(0, 8)) lines.push(`  - ${e.tool}: ${e.error}`);
}
if (process.matchedLog) lines.push(`- 任务日志：\`${process.matchedLog}\``);
if (timing) lines.push(`- 日志大小：${timing.bytes} bytes`);
if (existsSync(checkpointDir(harnessRoot))) {
  lines.push("- Checkpoints：");
  for (const stage of ["code", "office", "integration"]) {
    const cp = readComplexCheckpoint(harnessRoot, stage);
    if (cp) lines.push(`  - ${stage}: ${cp.at}`);
  }
}
lines.push("");

lines.push("## 6. 优化方案（已落地 / 待做）");
lines.push("");
lines.push("### 已落地");
lines.push("- brief contract 收紧（Word/HTML/QA/manifest 关键证据）");
lines.push("- complex_verify 语义校验 + score clamp 100");
lines.push("- manifest code_repo/git_repo 别名");
lines.push("- checkpoint 与 audit 工具链回填");
lines.push("");
lines.push("### 待做（v3）");
lines.push("- 30 天 CSV + 脚本聚合 + 1% 数据冲突 reconciliation");
lines.push("- 第二仓库只读 API，HTML 需 fetch JSON");
lines.push("- office-writer / general-purpose 分轨并行");
lines.push("");
lines.push("## 7. 复现");
lines.push("```bash");
lines.push("cd test/e2e-delivery-chain");
lines.push("SKIP_RESET=1 COMPLEX_ONLY=1 E2E_STRICT_QUALITY=1 node run_all.mjs");
lines.push("node generate_complex_audit.mjs");
lines.push("```");

const reportPath = join(outDir, `COMPLEX-AUDIT-${date}.md`);
writeFileSync(reportPath, lines.join("\n") + "\n");
console.log(reportPath);
spawnSync("node", [join(harnessRoot, "generate_report.mjs")], { stdio: "inherit", cwd: harnessRoot });
