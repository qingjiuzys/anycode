#!/usr/bin/env node
/**
 * Sync official skill catalog table into docs-site guide pages.
 * Source of truth: crates/dashboard/src/skill_market.rs (official_catalog_entries).
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const marketRs = fs.readFileSync(
  path.join(root, "crates/dashboard/src/skill_market.rs"),
  "utf8",
);

const entries = [];
const blockRe =
  /SkillMarketEntry\s*\{[^}]*id:\s*"([^"]+)"[^}]*name:\s*"([^"]+)"[^}]*category:\s*"([^"]+)"[^}]*source:\s*"([^"]+)"[^}]*badge:\s*"official"/gs;
for (const m of marketRs.matchAll(blockRe)) {
  entries.push({ id: m[1], name: m[2], category: m[3], source: m[4] });
}

if (entries.length === 0) {
  console.error("No official entries parsed from skill_market.rs");
  process.exit(1);
}

function tableMd(rows, lang) {
  const header =
    lang === "zh"
      ? "| ID | 分类 | 来源 |\n|----|------|------|"
      : "| ID | Category | Source |\n|----|----------|--------|";
  const body = rows
    .map((r) => `| ${r.id} | ${r.category} | ${r.source} |`)
    .join("\n");
  return `${header}\n${body}`;
}

function patchPage(file, lang) {
  let text = fs.readFileSync(file, "utf8");
  const table = tableMd(entries, lang);
  text = text.replace(/\| ID \|[^\n]+\n(?:\|[-| ]+\n)?(?:\|[^\n]+\n)+/m, `${table}\n`);
  fs.writeFileSync(file, text);
  console.log(`updated ${path.relative(root, file)} (${entries.length} rows)`);
}

patchPage(path.join(root, "docs-site/guide/skills/index.md"), "en");
patchPage(path.join(root, "docs-site/zh/guide/skills/index.md"), "zh");
