import { expect, test } from "@playwright/test";

test.describe("Agents tabs", () => {
  test("switches installed, catalog, and import tabs", async ({ page }) => {
    await page.goto("/agents");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();

    const catalogTab = page.getByRole("button", { name: /official|catalog/i }).first();
    if (await catalogTab.isVisible()) {
      await catalogTab.click();
      await expect(page.locator(".dw-card, .skill-market-panel, main")).toBeVisible();
    }

    const importTab = page.getByRole("button", { name: /import/i }).first();
    if (await importTab.isVisible()) {
      await importTab.click();
      await expect(page.getByRole("main")).toBeVisible();
    }
  });

  test("installed skills show Chinese names in zh locale", async ({ page, request }) => {
    const install = await request.post("/api/skills/install-starter");
    expect(install.ok(), await install.text()).toBeTruthy();
    const installed = (await install.json()) as { count?: number; installed?: string[] };
    expect(installed.count ?? 0).toBeGreaterThan(0);

    const skillsRes = await request.get("/api/skills?limit=100");
    expect(skillsRes.ok()).toBeTruthy();
    const skillsBody = (await skillsRes.json()) as { skills?: Array<{ id: string }> };
    const ids = new Set((skillsBody.skills ?? []).map((s) => s.id));
    expect(
      ids.has("cn-daily-brief") || ids.has("report-to-csv"),
      `starter skills missing from API after install: ${[...ids].slice(0, 12).join(",")}`,
    ).toBeTruthy();

    await page.addInitScript(() => {
      localStorage.setItem("anycode-dashboard-locale", "zh");
    });
    await page.goto("/agents");
    await page.locator("#agents-tab-installed").click();
    await expect(page.locator(".dw-agents-skill-row").first()).toBeVisible({
      timeout: 15_000,
    });

    // Force locale in case the shell remounted before init script applied.
    await page.evaluate(() => {
      localStorage.setItem("anycode-dashboard-locale", "zh");
    });
    await page.reload();
    await page.locator("#agents-tab-installed").click();

    const zhName = page.locator(".dw-agents-skill-row__name").filter({
      hasText: /中文日报|报表转 CSV/,
    });
    await expect(zhName.first()).toBeVisible({ timeout: 15_000 });
  });

  test("skills market API returns entries", async ({ request }) => {
    const res = await request.get("/api/skills/market");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.market?.entries)).toBeTruthy();
  });
});
