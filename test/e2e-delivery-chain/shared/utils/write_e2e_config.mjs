#!/usr/bin/env node
/** Merge user ~/.anycode/config.json with e2e harness overrides (turns, security bypass). */
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const outPath = process.argv[2] ?? join(harnessRoot, "out/e2e-anycode.config.json");
const userPath = join(homedir(), ".anycode/config.json");

let base = {};
if (existsSync(userPath)) {
  base = JSON.parse(readFileSync(userPath, "utf8"));
}

base.runtime = {
  ...(base.runtime ?? {}),
  max_agent_turns: Number(process.env.ANYCODE_MAX_AGENT_TURNS ?? 9999),
  max_tool_calls: Number(process.env.ANYCODE_MAX_TOOL_CALLS ?? 50_000),
};

base.security = {
  ...(base.security ?? {}),
  permission_mode: "bypass",
  require_approval: false,
  sandbox_mode: false,
};

base.skills = {
  ...(base.skills ?? {}),
  enabled: true,
};

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(base, null, 2) + "\n");
console.log(outPath);
