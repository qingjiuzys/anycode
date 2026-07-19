import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { runScenarioQualityVerify } from "../../shared/utils/verify_runner.mjs";
import { gatePass } from "../../shared/utils/quality_score.mjs";
import { PYTHON } from "../../shared/utils/python.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const workspace = readFileSync(join(harnessRoot, "out/workspace_path.txt"), "utf8").trim();
const verifyPy = join(harnessRoot, "shared/utils/complex_verify.py");
const runResult = JSON.parse(
  readFileSync(join(harnessRoot, "out/08-complex-delivery.json"), "utf8"),
);
const verifyArgs = [
  verifyPy,
  "--workspace", workspace,
  "--json",
  "--owner-file", join(harnessRoot, "out/.e2e-artifact-owner.json"),
];
if (runResult.evalRunId) verifyArgs.push("--run-id", runResult.evalRunId);
if (runResult.modelProfile) verifyArgs.push("--profile", runResult.modelProfile);
if (runResult.sessionId) verifyArgs.push("--session-id", runResult.sessionId);

const r = spawnSync(
  PYTHON,
  verifyArgs,
  { encoding: "utf8", cwd: harnessRoot },
);

let payload;
try {
  payload = JSON.parse(r.stdout);
} catch {
  payload = { pass: false, score: 0, grade: "FAIL", findings: [{ severity: "P0", message: r.stderr || r.stdout }] };
}

import { writeFileSync, mkdirSync } from "node:fs";
const outPath = join(harnessRoot, "out/08-verify.json");
mkdirSync(join(harnessRoot, "out"), { recursive: true });
writeFileSync(outPath, JSON.stringify(payload, null, 2) + "\n");

console.log(`score=${payload.score} grade=${payload.grade}`);
process.exit(gatePass(payload) ? 0 : 1);
