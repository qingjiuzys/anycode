#!/usr/bin/env node
/**
 * JSON-lines Playwright helper for dashboard workbench browser panel.
 */
import { createRequire } from "node:module";
import readline from "node:readline";

const require = createRequire(import.meta.url);
const playwright = require("playwright");

let browser = null;
let page = null;

async function ensurePage() {
  if (page) return page;
  browser = await playwright.chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  page = await context.newPage();
  return page;
}

function ok(payload = {}) {
  process.stdout.write(`${JSON.stringify({ ok: true, ...payload })}\n`);
}

function fail(message) {
  process.stdout.write(`${JSON.stringify({ ok: false, error: String(message) })}\n`);
}

async function handle(cmd) {
  switch (cmd.cmd) {
    case "create": {
      const p = await ensurePage();
      ok({ state: { url: p.url(), title: await p.title() } });
      break;
    }
    case "navigate": {
      const p = await ensurePage();
      await p.goto(String(cmd.url || "about:blank"), {
        waitUntil: "domcontentloaded",
        timeout: 30_000,
      });
      ok({ state: { url: p.url(), title: await p.title() } });
      break;
    }
    case "state": {
      if (!page) {
        ok({ state: { url: "about:blank", title: "" } });
        break;
      }
      ok({ state: { url: page.url(), title: await page.title() } });
      break;
    }
    case "screenshot": {
      const p = await ensurePage();
      const buf = await p.screenshot({ type: "png", fullPage: false });
      ok({
        image_base64: buf.toString("base64"),
        viewport: { width: 1280, height: 720 },
      });
      break;
    }
    case "close": {
      if (browser) await browser.close();
      browser = null;
      page = null;
      ok();
      break;
    }
    default:
      fail(`unknown cmd: ${cmd.cmd}`);
  }
}

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let cmd;
  try {
    cmd = JSON.parse(trimmed);
  } catch (e) {
    fail(e.message);
    return;
  }
  handle(cmd).catch((e) => fail(e.message));
});

process.on("SIGTERM", async () => {
  if (browser) await browser.close();
  process.exit(0);
});
