import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { findNewest } from "./files.mjs";
import { runQualityVerify, gatePass } from "./quality_score.mjs";
import { auditScenarioRun } from "./process_audit.mjs";
import { capturePostRunAudit } from "./capture_process_audit.mjs";

export function resolveArtifact({ artifacts, preferred, ext }) {
  if (preferred && existsSync(preferred)) return preferred;
  return findNewest(artifacts, ext);
}

export function runScenarioQualityVerify({
  harnessRoot,
  scenarioId,
  kind,
  preferredPath,
  ext,
  auditRelative,
  runJsonPath,
  outVerifyName,
}) {
  const workspace = readFileSync(join(harnessRoot, "out/workspace_path.txt"), "utf8").trim();
  const artifacts = join(workspace, "artifacts");
  const fixturesDir = join(workspace, "fixtures");
  const path = resolveArtifact({ artifacts, preferred: preferredPath ? join(artifacts, preferredPath) : null, ext });
  if (!path) {
    const fail = { pass: false, score: 0, grade: "FAIL", findings: [{ severity: "P0", code: "missing_artifact", message: `no ${ext} found` }] };
    return { payload: fail, exitCode: 1 };
  }

  let process = {};
  const runJsonFile = runJsonPath ?? join(harnessRoot, "out", `${scenarioId}.json`);
  if (existsSync(runJsonFile)) {
    try {
      const runJson = JSON.parse(readFileSync(runJsonFile, "utf8"));
      if (scenarioId === "07-html-md-skill" && !existsSync(join(artifacts, "report_table.audit.json"))) {
        capturePostRunAudit({ harnessRoot, scenarioId, runJson, workspace });
        process = JSON.parse(readFileSync(runJsonFile, "utf8")).processAudit ?? auditScenarioRun(runJson);
      } else {
        process = runJson.processAudit ?? auditScenarioRun(runJson);
      }
    } catch {
      /* ignore */
    }
  }

  const auditPath = auditRelative ? join(artifacts, auditRelative) : null;
  const outPath = join(harnessRoot, "out", outVerifyName ?? `${scenarioId.split("-")[0]}-verify.json`);

  return runQualityVerify({
    kind,
    artifactPath: path,
    outPath,
    fixturesDir,
    auditPath: auditPath && existsSync(auditPath) ? auditPath : null,
    process,
  });
}
