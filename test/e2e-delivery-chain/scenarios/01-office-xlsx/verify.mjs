import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { runScenarioQualityVerify } from "../../shared/utils/verify_runner.mjs";
import { gatePass } from "../../shared/utils/quality_score.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const { exitCode, payload } = runScenarioQualityVerify({
  harnessRoot,
  scenarioId: "01-office-xlsx",
  kind: "xlsx",
  preferredPath: "june_sales.xlsx",
  ext: ".xlsx",
});
console.log(`score=${payload.score} grade=${payload.grade}`);
process.exit(gatePass(payload) ? 0 : 1);
