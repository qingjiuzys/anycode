import { readFileSync, existsSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { runScenarioQualityVerify } from "../../shared/utils/verify_runner.mjs";
import { gatePass } from "../../shared/utils/quality_score.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const workspace = readFileSync(join(harnessRoot, "out/workspace_path.txt"), "utf8").trim();
const artifacts = join(workspace, "artifacts");

const candidates = readdirSync(artifacts)
  .filter((n) => n.includes("report_table") && (n.endsWith(".pdf") || n.endsWith(".html")))
  .map((n) => join(artifacts, n));
const artifactPath = candidates[0];
const kind = artifactPath?.endsWith(".pdf") ? "pdf" : "html-export";

const { payload } = runScenarioQualityVerify({
  harnessRoot,
  scenarioId: "07-html-md-skill",
  kind,
  preferredPath: null,
  ext: artifactPath?.endsWith(".pdf") ? ".pdf" : ".html",
  auditRelative: "report_table.audit.json",
});

console.log(`score=${payload.score} grade=${payload.grade}`);
process.exit(gatePass(payload) ? 0 : 1);
