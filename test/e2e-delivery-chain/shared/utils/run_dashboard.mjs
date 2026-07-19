import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, existsSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { writeResult } from "./assert.mjs";
import { startConversation, waitForSession } from "./api.mjs";
import { ensureDashboard } from "./ensure_dashboard.mjs";
import { buildBrief } from "./build_brief.mjs";
import { capturePostRunAudit } from "./capture_process_audit.mjs";
import { writeComplexCheckpoint, snapshotWorkspaceStageSync } from "./complex_checkpoints.mjs";

export async function runDashboardScenario({ harnessRoot, scenarioId, promptPath, agent = "office-writer" }) {
  await ensureDashboard();
  const outDir = join(harnessRoot, "out");
  const projectId = readFileSync(join(outDir, "project_id.txt"), "utf8").trim();
  const workspace = readFileSync(join(outDir, "workspace_path.txt"), "utf8").trim();
  const userPrompt = readFileSync(promptPath, "utf8").trim();
  const { brief, briefPath, spec } = buildBrief({ scenarioId, userPrompt, harnessRoot });
  const profile = process.env.E2E_MODEL_PROFILE ?? "agnes";
  const maxTurns = process.env.ANYCODE_MAX_AGENT_TURNS ?? "9999";
  const maxTools = process.env.ANYCODE_MAX_TOOL_CALLS ?? "50000";

  console.log(
    `[${scenarioId}] dashboard runtime agent=${agent} profile=${profile} turns=${maxTurns} (brief -> ${briefPath})`,
  );
  const startedAt = new Date().toISOString();
  const startedAtMs = Date.now();
  const started = await startConversation(projectId, brief, agent);
  const sessionId = started?.session?.id;
  if (!sessionId) {
    throw new Error(`dashboard did not return a session id: ${JSON.stringify(started)}`);
  }
  const session = await waitForSession(sessionId, {
    timeoutMs: Number(process.env.E2E_SESSION_TIMEOUT_MS ?? 900_000),
  });
  const runExit = session.status === "completed" ? 0 : 1;
  const result = {
    scenario: scenarioId,
    sessionId,
    status: session.status,
    trusted_status: session?.trusted_status ?? "unverified",
    runExit,
    stdoutTail: null,
    stderrTail: null,
    mode: "dashboard_embedded_runtime",
    briefPath,
    userPromptLength: userPrompt.length,
    workspace,
    evalRunId: process.env.E2E_EVAL_RUN_ID ?? null,
    modelProfile: profile,
    startedAt,
  };
  writeResult(join(outDir, `${scenarioId}.json`), result);

  if (runExit !== 0) {
    throw new Error(`dashboard session ended with ${session.status}`);
  }

  capturePostRunAudit({ harnessRoot, scenarioId, runJson: result, workspace });

  if (scenarioId === "08-complex-delivery") {
    const ownedArtifacts = spec.expected_artifacts
      .map((relativePath) => {
        const path = join(workspace, relativePath);
        if (!existsSync(path) || !statSync(path).isFile()) return null;
        const stat = statSync(path);
        return {
          path: relativePath,
          sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
          mtimeMs: stat.mtimeMs,
          fresh: stat.mtimeMs >= startedAtMs,
        };
      })
      .filter(Boolean);
    const owner = {
      evalRunId: result.evalRunId,
      modelProfile: profile,
      sessionId: result.sessionId,
      workspace,
      startedAt,
      artifacts: ownedArtifacts,
    };
    const ownerJson = JSON.stringify(owner, null, 2) + "\n";
    writeFileSync(join(outDir, ".e2e-artifact-owner.json"), ownerJson);
    writeFileSync(join(workspace, "artifacts/.e2e-artifact-owner.json"), ownerJson);
    const snap = snapshotWorkspaceStageSync(workspace, spawnSync);
    writeComplexCheckpoint(harnessRoot, "code", { snapshot: snap.code, runExit });
    writeComplexCheckpoint(harnessRoot, "office", { snapshot: snap.office, runExit });
    writeComplexCheckpoint(harnessRoot, "integration", { snapshot: snap.integration, runExit });
  }

  console.log(`[${scenarioId}] completed exit=0 session=${result.sessionId ?? "n/a"}`);
  return result;
}
