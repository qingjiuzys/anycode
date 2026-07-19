#!/usr/bin/env node
/** Register e2e workspace with dashboard; writes out/project_id.txt and project.json. */
import { writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { upsertProject, health } from "./api.mjs";

const harnessRoot = join(dirname(fileURLToPath(import.meta.url)), "../..");
const outDir = join(harnessRoot, "out");
const workspace = process.env.E2E_WORKSPACE ?? join(process.env.HOME ?? "", ".anycode/workspace/e2e-delivery");
const name = process.env.E2E_PROJECT_NAME ?? "e2e-delivery";

await health();
const project = await upsertProject({
  root_path: workspace,
  name,
  create_root: false,
});

mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "project_id.txt"), `${project.id}\n`);
writeFileSync(join(outDir, "workspace_path.txt"), `${workspace}\n`);
writeFileSync(join(outDir, "project.json"), `${JSON.stringify({ project }, null, 2)}\n`);
console.log(`bootstrap complete project_id=${project.id} workspace=${workspace}`);
