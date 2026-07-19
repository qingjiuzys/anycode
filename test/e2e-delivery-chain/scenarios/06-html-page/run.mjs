import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { runDashboardScenario } from "../../shared/utils/run_dashboard.mjs";

const __dir = dirname(fileURLToPath(import.meta.url));
const harnessRoot = join(__dir, "../..");
await runDashboardScenario({
  harnessRoot,
  scenarioId: "06-html-page",
  promptPath: join(__dir, "prompt.md"),
  agent: "general-purpose",
});
