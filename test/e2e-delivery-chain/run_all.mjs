#!/usr/bin/env node
/**
 * E2E delivery chain orchestrator: reset → bootstrap → scenarios 01–07 → report.
 */
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, existsSync, readdirSync, rmSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const harnessRoot = __dir;
const outDir = join(harnessRoot, "out");
const date = new Date().toISOString().slice(0, 10);

if (process.env.COMPLEX_ONLY === "1" && process.env.VERIFY_ONLY !== "1" && process.env.REPORT_ONLY !== "1") {
  for (const name of ["08-complex-delivery.json", "08-verify.json", ".e2e-artifact-owner.json"]) {
    rmSync(join(outDir, name), { force: true });
  }
}

const SCENARIOS = [
  { id: "01-office-xlsx", suite: "office", dir: "01-office-xlsx" },
  { id: "02-office-docx", suite: "office", dir: "02-office-docx" },
  { id: "03-office-pptx", suite: "office", dir: "03-office-pptx" },
  { id: "04-coding-mock", suite: "coding", dir: "04-coding-mock" },
  { id: "05-coding-live", suite: "coding", dir: "05-coding-live" },
  { id: "06-html-page", suite: "html", dir: "06-html-page" },
  { id: "07-html-md-skill", suite: "html", dir: "07-html-md-skill" },
  { id: "08-complex-delivery", suite: "complex", dir: "08-complex-delivery" },
];

function sh(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { encoding: "utf8", stdio: "inherit", ...opts });
  return r.status ?? 1;
}

function readJson(path) {
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function readText(path) {
  if (!existsSync(path)) return null;
  return readFileSync(path, "utf8").trim();
}

function maskConfig() {
  const p = join(process.env.HOME ?? "", ".anycode/config.json");
  if (!existsSync(p)) return "(missing)";
  try {
    const c = JSON.parse(readFileSync(p, "utf8"));
    const model = c?.default_model ?? c?.model ?? c?.providers?.[0]?.model ?? "unknown";
    return String(model).replace(/sk-[a-zA-Z0-9]+/g, "sk-***");
  } catch {
    return "(unreadable)";
  }
}

function runScenario(sc) {
  const base = join(harnessRoot, "scenarios", sc.dir);
  let runStatus = 0;
  const skipRun =
    process.env.VERIFY_ONLY === "1" ||
    process.env.COMPLEX_STAGE === "verify" ||
    (sc.id === "08-complex-delivery" && process.env.COMPLEX_STAGE === "verify");
  if (!skipRun) {
    console.log(`\n======== ${sc.id} RUN ========`);
    runStatus = sh("node", [join(base, "run.mjs")], { stdio: "inherit" });
  }
  console.log(`\n-------- ${sc.id} VERIFY --------`);
  const verifyStatus = sh("node", [join(base, "verify.mjs")], { stdio: "inherit" });
  const verify = readJson(join(outDir, `${sc.id.split("-")[0]}-verify.json`));
  const strict = process.env.E2E_STRICT_QUALITY === "1";
  const p1Count = (verify?.findings ?? []).filter((f) => f.severity === "P0" || f.severity === "P1").length;
  const qualityPass = strict
    ? verify?.pass === true && verify?.grade === "PASS" && p1Count === 0
    : verify?.pass === true || (verify?.grade === "WARN" && (verify?.score ?? 0) >= 70);
  return {
    id: sc.id,
    suite: sc.suite,
    run: runStatus === 0 ? "PASS" : "FAIL",
    verify: verifyStatus === 0 ? "PASS" : "FAIL",
    score: verify?.score,
    grade: verify?.grade,
    pass: (process.env.VERIFY_ONLY === "1" ? verifyStatus === 0 : runStatus === 0) && qualityPass,
  };
}

if (process.env.REPORT_ONLY === "1" || process.env.VERIFY_ONLY === "1") {
  const verifyScenarios =
    process.env.COMPLEX_ONLY === "1"
      ? SCENARIOS.filter((s) => s.id === "08-complex-delivery")
      : process.env.RUN_COMPLEX === "1"
        ? SCENARIOS
        : SCENARIOS.filter((s) => s.id !== "08-complex-delivery");
  if (process.env.VERIFY_ONLY === "1") {
    let strictFail = 0;
    for (const sc of verifyScenarios) {
      const st = sh("node", [join(harnessRoot, "scenarios", sc.dir, "verify.mjs")]);
      if (st !== 0) strictFail++;
      if (process.env.E2E_STRICT_QUALITY === "1") {
        const v = readJson(join(outDir, `${sc.id.split("-")[0]}-verify.json`));
        if (v?.grade === "WARN" || v?.grade === "FAIL") strictFail++;
        const p1 = (v?.findings ?? []).filter((f) => f.severity === "P0" || f.severity === "P1").length;
        if (p1 > 0) strictFail++;
      }
    }
    if (verifyScenarios.some((s) => s.id === "08-complex-delivery")) {
      sh("node", [join(harnessRoot, "generate_complex_audit.mjs")]);
    }
    sh("node", [join(harnessRoot, "generate_report.mjs")]);
    process.exit(strictFail > 0 ? 1 : 0);
  }
  sh("node", [join(harnessRoot, "generate_report.mjs")]);
  process.exit(0);
}

if (process.env.SKIP_RESET !== "1") {
  console.log("==> Phase 0 reset");
  if (sh("bash", [join(harnessRoot, "reset.sh")]) !== 0) process.exit(1);
}

if (process.env.SKIP_BOOTSTRAP !== "1") {
  console.log("==> Phase 1 bootstrap");
  if (sh("bash", [join(harnessRoot, "bootstrap.sh")]) !== 0) process.exit(1);
} else {
  console.log("==> Phase 1 bootstrap (skipped)");
}

if (process.env.E2E_WORKSPACE) {
  const registeredWorkspace = readText(join(outDir, "workspace_path.txt"));
  if (registeredWorkspace !== process.env.E2E_WORKSPACE) {
    console.error(`workspace mismatch: registered=${registeredWorkspace} expected=${process.env.E2E_WORKSPACE}`);
    process.exit(1);
  }
}

const scenariosToRun =
  process.env.COMPLEX_ONLY === "1"
    ? SCENARIOS.filter((s) => s.id === "08-complex-delivery")
    : process.env.RUN_COMPLEX === "1"
      ? SCENARIOS
      : SCENARIOS.filter((s) => s.id !== "08-complex-delivery");

const results = [];
for (const sc of scenariosToRun) {
  try {
    results.push(runScenario(sc));
  } catch (e) {
    results.push({ id: sc.id, suite: sc.suite, run: "FAIL", verify: "FAIL", pass: false, error: String(e) });
  }
}

if (scenariosToRun.some((s) => s.id === "08-complex-delivery")) {
  sh("node", [join(harnessRoot, "generate_complex_audit.mjs")]);
}

const suites = { office: [], coding: [], html: [], complex: [] };
for (const r of results) suites[r.suite].push(r);

function suiteSummary(rows) {
  const pass = rows.filter((x) => x.pass).length;
  const fail = rows.filter((x) => !x.pass).length;
  return { pass, fail, skip: 0, total: rows.length };
}

sh("node", [join(harnessRoot, "generate_report.mjs")]);

const failed = results.filter((r) => !r.pass).length;
process.exit(failed > 0 ? 1 : 0);
