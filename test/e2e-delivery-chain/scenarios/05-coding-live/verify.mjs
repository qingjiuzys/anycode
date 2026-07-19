import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const data = JSON.parse(readFileSync(join(harnessRoot, "out/05-coding-live.json"), "utf8"));
if (data.skipped) {
  console.log("live coding skipped (LLM unavailable)");
  process.exit(0);
}
process.exit(data.pass ? 0 : 1);
