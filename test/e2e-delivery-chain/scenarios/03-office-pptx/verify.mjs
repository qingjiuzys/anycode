import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { runScenarioQualityVerify } from "../../shared/utils/verify_runner.mjs";
import { gatePass } from "../../shared/utils/quality_score.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const { payload } = runScenarioQualityVerify({
  harnessRoot,
  scenarioId: "03-office-pptx",
  kind: "pptx",
  preferredPath: "june_sales_deck.pptx",
  ext: ".pptx",
});
console.log(`score=${payload.score} grade=${payload.grade}`);
process.exit(gatePass(payload) ? 0 : 1);
