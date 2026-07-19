import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import {
  auditScenarioRun,
  buildProcessAuditRecord,
  writeProcessAuditFile,
  extractTaskLogPath,
  auditFromTaskLog,
} from "./process_audit.mjs";
import { getScenarioSpec } from "./build_brief.mjs";

/**
 * After Agent run: capture process evidence from task logs and write audit files.
 */
export function capturePostRunAudit({ harnessRoot, scenarioId, runJson, workspace }) {
  let process = auditScenarioRun(runJson);
  const logFromStderr = extractTaskLogPath(`${runJson.stderrTail ?? ""}\n${runJson.stdoutTail ?? ""}`);
  if (logFromStderr && existsSync(logFromStderr)) {
    process = auditFromTaskLog(logFromStderr);
  }

  const enriched = { ...runJson, processAudit: process };
  const spec = getScenarioSpec(scenarioId);
  const artifactsDir = join(workspace, "artifacts");

  if (scenarioId === "07-html-md-skill" || spec?.artifact_type === "pdf") {
    const pdf = join(artifactsDir, "report_table.pdf");
    const html = join(artifactsDir, "report_table.html");
    const actual = existsSync(pdf) ? pdf : existsSync(html) ? html : null;
    const auditPath = join(artifactsDir, "report_table.audit.json");
    const record = buildProcessAuditRecord({
      scenarioId,
      process,
      artifactPath: actual,
      workspace,
    });
    writeProcessAuditFile(auditPath, record);
    enriched.processAuditPath = auditPath;
  }

  writeFileSync(join(harnessRoot, "out", `${scenarioId}.json`), JSON.stringify(enriched, null, 2) + "\n");
  return enriched;
}
