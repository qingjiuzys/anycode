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
    await page.addInitScript(() => {
      localStorage.setItem("anycode-dashboard-locale", "zh");
    });
    // Fixture HOME may have an empty skills dir — seed starter pack first.
    const install = await request.post("/api/skills/install-starter");
    expect(install.ok()).toBeTruthy();
    await page.goto("/agents");
    // Default tab is already "installed"; use tab id (avoid matching other「已安装」buttons).
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
