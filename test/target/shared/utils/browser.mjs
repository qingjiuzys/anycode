import { createRequire } from "node:module";
import path from "node:path";
import { repoRoot } from "./files.mjs";

const require = createRequire(import.meta.url);
const playwright = require(path.join(repoRoot, "crates/dashboard-ui/node_modules/playwright"));

export async function withPage(fn) {
  const browser = await playwright.chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  try {
    return await fn(page);
  } finally {
    await browser.close();
  }
}

export async function bodyText(page) {
  return await page.locator("body").innerText({ timeout: 10_000 });
}

export function hasFatalText(text) {
  return /uncaught|exception|panic|fatal error|failed to fetch/i.test(text);
}
