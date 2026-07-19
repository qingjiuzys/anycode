import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const data = JSON.parse(readFileSync(join(harnessRoot, "out/04-coding-mock.json"), "utf8"));
process.exit(data.pass ? 0 : 1);
