import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { runDashboardScenario } from "../../shared/utils/run_dashboard.mjs";

const __dir = dirname(fileURLToPath(import.meta.url));
const harnessRoot = join(__dir, "../..");

process.env.E2E_SESSION_TIMEOUT_MS = process.env.E2E_SESSION_TIMEOUT_MS ?? "7200000";

await runDashboardScenario({
  harnessRoot,
  scenarioId: "08-complex-delivery",
  promptPath: join(__dir, "prompt.md"),
  agent: "general-purpose",
});
