import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

export function fail(msg) {
  const err = new Error(msg);
  err.isAssert = true;
  throw err;
}

export function ok(cond, msg) {
  if (!cond) fail(msg);
}

export function writeResult(outPath, payload) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(payload, null, 2) + "\n", "utf8");
}
