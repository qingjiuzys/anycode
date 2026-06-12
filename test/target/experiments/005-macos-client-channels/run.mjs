import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded, pass } from "../../shared/utils/assert.mjs";
import { bodyText, hasFatalText, withPage } from "../../shared/utils/browser.mjs";
import { targetRoot, write } from "../../shared/utils/files.mjs";

const id = "005-macos-client-channels";
const outDir = path.join(targetRoot, "experiments", id, "out");
const baseUrl = process.env.ANYCODE_DASHBOARD_URL ?? "http://127.0.0.1:43180";

function hasAny(text, needles) {
  return needles.some((needle) => text.includes(needle));
}

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  await withPage(async (page) => {
    await page.goto(`${baseUrl}/settings?section=channels`, { waitUntil: "domcontentloaded", timeout: 15_000 });
    await page.waitForTimeout(800);
    const initial = await bodyText(page);
    assert(result, hasAny(initial, ["消息渠道", "Messaging channels", "Channels"]), "channels settings visible");
    assert(result, hasAny(initial, ["微信", "WeChat"]) && hasAny(initial, ["已配置", "configured"]), "wechat configured tab visible");
    assert(result, !hasFatalText(initial), "settings page has no fatal text");
    if (initial.includes("Bot Token")) {
      pass(result, "known_issue: channels route defaults to Telegram panel", { issue: "settings?section=channels does not preselect WeChat panel" });
    }
    await page.getByRole("button", { name: /微信|WeChat/ }).click();
    await page.waitForTimeout(600);
    const wechat = await bodyText(page);
    await page.screenshot({ path: path.join(outDir, "wechat_channels.png"), fullPage: true });
    addArtifact(result, path.join(outDir, "wechat_channels.png"), "wechat channels screenshot");
    assert(result, hasAny(wechat, ["本机已绑定微信账号", "This machine has a bound WeChat account", "WeChat account is linked on this machine"]), "local wechat account bound");
    assert(result, hasAny(wechat, ["打开微信设置", "Open WeChat settings", "Open WeChat setup"]), "open wechat settings button present");
    assert(result, !wechat.includes("Bot Token"), "wechat panel no longer shows telegram token field");
    write(path.join(outDir, "page_text.txt"), wechat);
  });
});
