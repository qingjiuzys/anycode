import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { registeredIconNames } from "./Icon";

const SRC_ROOT = join(import.meta.dirname, "..");

function walkFiles(dir: string, acc: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "dist") continue;
      walkFiles(path, acc);
      continue;
    }
    if (/\.(tsx?|jsx?)$/.test(entry.name)) {
      acc.push(path);
    }
  }
  return acc;
}

function collectIconNames(source: string): string[] {
  const names: string[] = [];
  const iconNamePattern = /\b(?:Icon\s+name=|icon=)\{?\s*["']([a-z0-9_]+)["']\s*\}?/g;
  for (const match of source.matchAll(iconNamePattern)) {
    names.push(match[1]!);
  }
  return names;
}

describe("Icon registry coverage", () => {
  it("registers every icon name used in dashboard-ui source", () => {
    const used = new Map<string, string[]>();
    for (const file of walkFiles(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, file);
      if (rel.endsWith("Icon.test.ts")) continue;
      for (const name of collectIconNames(readFileSync(file, "utf8"))) {
        const files = used.get(name) ?? [];
        files.push(rel);
        used.set(name, files);
      }
    }

    const missing = [...used.entries()].filter(([name]) => !registeredIconNames.has(name));
    expect(
      missing.map(([name, files]) => `${name} (${files.join(", ")})`),
      "missing icon SVG definitions",
    ).toEqual([]);
  });
});
