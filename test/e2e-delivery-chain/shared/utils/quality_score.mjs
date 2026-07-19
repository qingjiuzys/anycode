import { spawnSync } from "node:child_process";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { PYTHON } from "./python.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");

/**
 * Run office_verify.py --json and merge with optional process audit.
 */
export function runQualityVerify({ kind, artifactPath, outPath, fixturesDir, auditPath, process }) {
  const verifyPy = join(harnessRoot, "shared/utils/office_verify.py");
  const args = [verifyPy, kind, artifactPath, "--json", "--fixtures", fixturesDir];
  if (auditPath) args.push("--audit", auditPath);
  if (process && Object.keys(process).length) {
    args.push("--process-json", JSON.stringify(process));
  }

  const r = spawnSync(PYTHON, args, { encoding: "utf8" });

  let payload;
  try {
    payload = JSON.parse(r.stdout);
  } catch {
    payload = {
      pass: false,
      score: 0,
      grade: "FAIL",
      findings: [{ severity: "P0", code: "verify_crash", message: (r.stderr || r.stdout || "verify failed").slice(0, 500) }],
      artifact: artifactPath,
    };
  }

  if (payload.grade === "WARN") {
    payload.pass_with_warnings = true;
  }

  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(payload, null, 2) + "\n");
  const exitCode = gatePass(payload) ? 0 : 1;
  return { payload, exitCode };
}

export function gatePass(payload) {
  if (!payload) return false;
  if (payload.grade === "PASS") return true;
  if (payload.grade === "WARN" && (payload.score ?? 0) >= 70) {
    return process.env.E2E_STRICT_QUALITY !== "1";
  }
  return false;
}
