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

  test("installed skills show Chinese names in zh locale", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem("anycode-dashboard-locale", "zh");
    });
    await page.goto("/agents");
    await page.getByRole("button", { name: /已安装|installed/i }).first().click();
    const row = page.locator(".dw-agents-skill-row").filter({ hasText: "cn-daily-brief" }).first();
    if (await row.count()) {
      await expect(row.locator(".dw-agents-skill-row__name")).toHaveText("中文日报");
    } else {
      const csvRow = page.locator(".dw-agents-skill-row").filter({ hasText: "report-to-csv" }).first();
      await expect(csvRow.locator(".dw-agents-skill-row__name")).toHaveText("报表转 CSV");
    }
  });

  test("skills market API returns entries", async ({ request }) => {
    const res = await request.get("/api/skills/market");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.market?.entries)).toBeTruthy();
  });
});
