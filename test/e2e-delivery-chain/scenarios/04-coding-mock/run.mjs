import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const harnessRoot = join(__dir, "../..");
const root = join(harnessRoot, "../..");
const anycode = process.env.ANYCODE_BIN ?? join(root, "target/release/anycode");
const outDir = join(harnessRoot, "out");
mkdirSync(outDir, { recursive: true });

const r = spawnSync(anycode, ["eval", "run", "--mock", "--json"], {
  cwd: root,
  encoding: "utf8",
  timeout: 300_000,
});

const payload = {
  pass: false,
  exitCode: r.status,
  stdout: r.stdout?.slice(0, 20_000),
  stderr: r.stderr?.slice(0, 8_000),
};
if (r.status === 0 && r.stdout) {
  try {
    const rows = JSON.parse(r.stdout.trim().match(/\[[\s\S]*\]/)?.[0] ?? "[]");
    payload.rows = rows;
    payload.pass = Array.isArray(rows) && rows.length > 0 && rows.every((x) => x.status === "pass");
  } catch (e) {
    payload.parseError = String(e);
  }
}

writeFileSync(join(outDir, "04-coding-mock.json"), JSON.stringify(payload, null, 2) + "\n");
console.log(payload.pass ? "mock eval PASS" : "mock eval FAIL");
process.exit(payload.pass ? 0 : 1);
