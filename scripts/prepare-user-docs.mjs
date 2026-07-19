#!/usr/bin/env node
/**
 * Stage user docs from docs/user/ into account-portal public/docs-src/.
 * Strips VitePress frontmatter and rewrites internal doc links for /docs/* routes.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const srcRoot = path.join(root, "docs/user");
const outRoot = path.join(root, "crates/account-portal/public/docs-src");

function stripFrontmatter(text) {
  if (!text.startsWith("---\n")) return text;
  const end = text.indexOf("\n---\n", 4);
  if (end === -1) return text;
  return text.slice(end + 5);
}

function rewriteLinks(text) {
  return text
    .replace(/<HelpContact[^>]*\/>/g, "")
    .replace(/<HomeExtras\s*\/>/g, "")
    .replace(/\]\(\/zh\/([^)]+)\)/g, "](/docs/zh/$1)")
    .replace(/\]\(\/guide\//g, "](/docs/guide/")
    .replace(/\]\(\/help\)/g, "](/docs/help)")
    .replace(/\]\(\/zh\/help\)/g, "](/docs/zh/help)");
}

function copyMd(srcPath, destPath) {
  const raw = fs.readFileSync(srcPath, "utf8");
  const cleaned = rewriteLinks(stripFrontmatter(raw));
  fs.mkdirSync(path.dirname(destPath), { recursive: true });
  fs.writeFileSync(destPath, cleaned);
}

function walk(dir, rel = "") {
  for (const name of fs.readdirSync(dir)) {
    const abs = path.join(dir, name);
    const nextRel = rel ? `${rel}/${name}` : name;
    if (fs.statSync(abs).isDirectory()) {
      walk(abs, nextRel);
      continue;
    }
    if (!name.endsWith(".md")) continue;
    copyMd(abs, path.join(outRoot, nextRel));
  }
}

if (!fs.existsSync(srcRoot)) {
  console.error(`missing docs source: ${srcRoot}`);
  process.exit(1);
}

fs.rmSync(outRoot, { recursive: true, force: true });
walk(srcRoot);
console.log(`staged user docs → ${path.relative(root, outRoot)}`);
