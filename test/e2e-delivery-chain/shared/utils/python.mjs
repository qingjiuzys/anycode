import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const pyFile = join(harnessRoot, "out/python.txt");
export const PYTHON = existsSync(pyFile) ? readFileSync(pyFile, "utf8").trim() : "python3";
