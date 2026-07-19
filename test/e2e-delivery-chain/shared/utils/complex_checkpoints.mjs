import { mkdirSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const STAGES = ["code", "office", "integration"];

export function checkpointDir(harnessRoot) {
  return join(harnessRoot, "out/08-checkpoints");
}

export function writeComplexCheckpoint(harnessRoot, stage, payload) {
  const dir = checkpointDir(harnessRoot);
  mkdirSync(dir, { recursive: true });
  const body = { stage, at: new Date().toISOString(), ...payload };
  writeFileSync(join(dir, `${stage}.json`), JSON.stringify(body, null, 2) + "\n");
  return body;
}

export function readComplexCheckpoint(harnessRoot, stage) {
  const p = join(checkpointDir(harnessRoot), `${stage}.json`);
  if (!existsSync(p)) return null;
  try {
    return JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

export function snapshotWorkspaceStageSync(workspace, spawnSync) {
  const exists = (rel) => existsSync(join(workspace, rel));
  const repo = join(workspace, "fixtures/e2e-complex-repo");
  let cargoOk = false;
  if (existsSync(repo)) {
    const r = spawnSync("cargo", ["test", "--workspace", "--quiet"], { cwd: repo, encoding: "utf8" });
    cargoOk = r.status === 0;
  }
  return {
    code: { repo: exists("fixtures/e2e-complex-repo"), changelog: exists("fixtures/e2e-complex-repo/CHANGELOG.md"), cargoOk },
    office: {
      june_ops: exists("artifacts/executive_pack/june_ops.xlsx"),
      scorecard: exists("artifacts/executive_pack/regional_scorecard.xlsx"),
      docx: exists("artifacts/executive_pack/incident_brief.docx"),
      pptx: exists("artifacts/executive_pack/board_deck.pptx"),
    },
    integration: {
      qa: exists("artifacts/executive_pack/qa_signoff.md"),
      html: exists("artifacts/executive_dashboard.html"),
      manifest: exists("artifacts/DELIVERY_MANIFEST.json"),
    },
  };
}

export function inferFailedStages(snapshot) {
  const failed = [];
  if (!snapshot.code.cargoOk) failed.push("code");
  if (!Object.values(snapshot.office).every(Boolean)) failed.push("office");
  if (!Object.values(snapshot.integration).every(Boolean)) failed.push("integration");
  return failed;
}

export { STAGES };
