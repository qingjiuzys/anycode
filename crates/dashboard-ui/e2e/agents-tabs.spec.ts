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

  test("skill market API returns entries", async ({ request }) => {
    const res = await request.get("/api/skills/market");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.entries)).toBeTruthy();
  });
});
