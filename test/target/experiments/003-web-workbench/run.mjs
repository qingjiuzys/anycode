import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded, pass } from "../../shared/utils/assert.mjs";
import { bodyText, hasFatalText, withPage } from "../../shared/utils/browser.mjs";
import { targetRoot, write } from "../../shared/utils/files.mjs";

const id = "003-web-workbench";
const outDir = path.join(targetRoot, "experiments", id, "out");
const baseUrls = process.env.ANYCODE_DASHBOARD_URL
  ? [process.env.ANYCODE_DASHBOARD_URL]
  : ["http://127.0.0.1:43180", "http://127.0.0.1:5174"];

function hasAny(text, needles) {
  return needles.some((needle) => text.includes(needle));
}

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  await withPage(async (page) => {
    let verified = false;
    for (const baseUrl of baseUrls) {
      await page.goto(`${baseUrl}/`, { waitUntil: "domcontentloaded", timeout: 15_000 });
      await page.waitForTimeout(800);
      const text = await bodyText(page);
      const suffix = baseUrl.replace(/[^0-9a-z]/gi, "_");
      await page.screenshot({ path: path.join(outDir, `home_${suffix}.png`), fullPage: true });
      addArtifact(result, path.join(outDir, `home_${suffix}.png`), `home screenshot ${baseUrl}`);
      const hasWorkbench = hasAny(text, ["最近继续", "Continue"])
        && hasAny(text, ["待处理", "Needs attention"])
        && hasAny(text, ["扫描项目", "新建项目", "Scan projects", "New project"]);
      if (!hasWorkbench) {
        pass(result, `known_issue: workbench missing on ${baseUrl}`, { url: baseUrl });
        continue;
      }
      assert(result, hasAny(text, ["我们应该在 anycode 中做些什么", "What should we do in anycode"]) || (await page.locator("textarea").count()) > 0, "home composer present", { url: baseUrl });
      assert(result, hasAny(text, ["最近继续", "Continue"]), "recent continue present", { url: baseUrl });
      assert(result, hasAny(text, ["待处理", "Needs attention"]), "pending work present", { url: baseUrl });
      assert(result, hasAny(text, ["扫描项目", "新建项目", "Scan projects", "New project"]), "quick actions present", { url: baseUrl });
      assert(result, !hasFatalText(text), "home has no fatal text", { url: baseUrl });
      await page.getByText(/查看报告|View reports/, { exact: true }).click();
      await page.waitForTimeout(800);
      const reportText = await bodyText(page);
      await page.screenshot({ path: path.join(outDir, `reports_${suffix}.png`), fullPage: true });
      addArtifact(result, path.join(outDir, `reports_${suffix}.png`), `reports screenshot ${baseUrl}`);
      assert(result, page.url().includes("/reports"), "view reports navigates to reports", { url: page.url() });
      assert(result, hasAny(reportText, ["报告", "Reports"]), "reports page text present");
      write(path.join(outDir, "page_text.txt"), text);
      verified = true;
      break;
    }
    assert(result, verified, "workbench verified on at least one dashboard target", { checked: baseUrls });
  });
});
