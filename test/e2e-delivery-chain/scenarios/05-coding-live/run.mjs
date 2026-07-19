import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync, cpSync, rmSync, existsSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const harnessRoot = join(__dir, "../..");
const repoRoot = join(harnessRoot, "../..");
const anycode = process.env.ANYCODE_BIN ?? join(repoRoot, "target/release/anycode");
const workspace = readFileSync(join(harnessRoot, "out/workspace_path.txt"), "utf8").trim();
const liveDir = join(workspace, "artifacts", "bugfix-live");
const src = join(repoRoot, "scripts/eval/fixtures/bugfix-repo");
const outDir = join(harnessRoot, "out");
mkdirSync(outDir, { recursive: true });

rmSync(liveDir, { recursive: true, force: true });
cpSync(src, liveDir, { recursive: true });

const prompt = "Fix the add function so unit tests pass";
const run = spawnSync(
  anycode,
  ["run", "-C", liveDir, prompt, "--ignore-approval"],
  {
    cwd: liveDir,
    encoding: "utf8",
    timeout: Number(process.env.E2E_LIVE_TIMEOUT_MS ?? 600_000),
    env: { ...process.env, ANYCODE_IGNORE_APPROVAL: "1" },
  },
);

let cargo = { status: 1, stdout: "", stderr: "skipped" };
if (run.status === 0 || existsSync(join(liveDir, "src/lib.rs"))) {
  cargo = spawnSync("cargo", ["test"], { cwd: liveDir, encoding: "utf8", timeout: 120_000 });
}

let taskEnd = false;
const tasksDir = join(process.env.HOME ?? "", ".anycode/tasks");
if (existsSync(tasksDir)) {
  for (const f of readdirSync(tasksDir)) {
    if (!f.endsWith(".log")) continue;
    const text = readFileSync(join(tasksDir, f), "utf8");
    if (text.includes("[task_end] status=completed")) taskEnd = true;
  }
}

const pass = cargo.status === 0;
const payload = {
  pass,
  skipped: false,
  runExit: run.status,
  cargoExit: cargo.status,
  taskEndSeen: taskEnd,
  stdout: run.stdout?.slice(-4000),
  stderr: run.stderr?.slice(-4000),
  cargoStdout: cargo.stdout?.slice(-2000),
  cargoStderr: cargo.stderr?.slice(-2000),
  liveDir,
};

if (run.status !== 0 && cargo.status !== 0) {
  payload.skipped = process.env.E2E_SKIP_LIVE_ON_FAIL === "1";
  if (payload.skipped) payload.pass = false;
}

writeFileSync(join(outDir, "05-coding-live.json"), JSON.stringify(payload, null, 2) + "\n");
console.log(pass ? "live coding PASS" : "live coding FAIL");
process.exit(pass ? 0 : payload.skipped ? 0 : 1);
