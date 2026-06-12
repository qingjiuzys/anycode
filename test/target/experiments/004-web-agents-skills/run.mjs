import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded } from "../../shared/utils/assert.mjs";
import { bodyText, hasFatalText, withPage } from "../../shared/utils/browser.mjs";
import { targetRoot, write } from "../../shared/utils/files.mjs";

const id = "004-web-agents-skills";
const outDir = path.join(targetRoot, "experiments", id, "out");
const baseUrl = process.env.ANYCODE_DASHBOARD_URL ?? "http://127.0.0.1:43180";

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  await withPage(async (page) => {
    await page.goto(`${baseUrl}/agents`, { waitUntil: "domcontentloaded", timeout: 15_000 });
    await page.waitForTimeout(800);
    const initial = await bodyText(page);
    assert(result, initial.includes("Agent / Skills"), "agents title present");
    assert(result, initial.includes("SKILLS") || initial.includes("Skills"), "skills summary present");
    assert(result, !hasFatalText(initial), "agents page has no fatal text");
    const search = page.getByPlaceholder(/搜索技能名称或描述|Search skill name or description|Search skills/).first();
    await search.fill("csv");
    await page.waitForTimeout(600);
    const filtered = await bodyText(page);
    await page.screenshot({ path: path.join(outDir, "agents_skills_csv.png"), fullPage: true });
    addArtifact(result, path.join(outDir, "agents_skills_csv.png"), "agents skills screenshot");
    assert(result, filtered.includes("report-to-csv"), "csv search finds report-to-csv");
    assert(result, !hasFatalText(filtered), "filtered page has no fatal text");
    write(path.join(outDir, "page_text.txt"), filtered);
  });
});
