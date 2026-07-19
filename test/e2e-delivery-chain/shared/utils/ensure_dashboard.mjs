import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { apiBase } from "./api.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../../../..");
const dashboardServe =
  process.env.ANYCODE_DASHBOARD_BIN ?? join(repoRoot, "target/release/anycode-dashboard-serve");

function healthy() {
  const r = spawnSync("curl", ["-sf", `${apiBase()}/api/health`], { encoding: "utf8" });
  return r.status === 0;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function ensureDashboard() {
  if (healthy()) return;
  spawnSync("pkill", ["-f", "anycode dashboard"]);
  spawnSync("pkill", ["-f", "anycode-dashboard-serve"]);
  await sleep(1000);
  if (!existsSync(dashboardServe)) {
    throw new Error(`dashboard runtime binary missing: ${dashboardServe}`);
  }
  const workspace = process.env.E2E_WORKSPACE;
  if (!workspace) {
    throw new Error("E2E_WORKSPACE is required to isolate dashboard HOME and config");
  }
  const runtimeHome = process.env.E2E_RUNTIME_HOME ?? `${workspace}-runtime-home`;
  spawnSync("bash", [
    "-c",
    `HOME="${runtimeHome}" ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1 ANYCODE_DASHBOARD_RECORD=1 ANYCODE_IGNORE_APPROVAL=1 nohup "${dashboardServe}" --host 127.0.0.1 --port 43180 >> /tmp/anycode-dashboard-e2e.log 2>&1 &`,
  ]);
  for (let i = 0; i < 90; i++) {
    if (healthy()) return;
    await sleep(1000);
  }
  throw new Error(`dashboard not healthy at ${apiBase()}`);
}
