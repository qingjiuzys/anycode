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

  test("install-starter syncs cn skills to API", async ({ request }) => {
    const install = await request.post("/api/skills/install-starter");
    expect(install.ok(), await install.text()).toBeTruthy();
    const installed = (await install.json()) as { count?: number; installed?: string[] };
    expect(installed.count ?? 0).toBeGreaterThan(0);
    expect(installed.installed ?? []).toEqual(
      expect.arrayContaining(["cn-daily-brief", "report-to-csv"]),
    );

    const skillsRes = await request.get("/api/skills?limit=100");
    expect(skillsRes.ok()).toBeTruthy();
    const skillsBody = (await skillsRes.json()) as { skills?: Array<{ id: string }> };
    const ids = new Set((skillsBody.skills ?? []).map((s) => s.id));
    expect(ids.has("cn-daily-brief")).toBeTruthy();
    expect(ids.has("report-to-csv")).toBeTruthy();
  });

  test("agents page loads installed tab after starter install", async ({ page, request }) => {
    await request.post("/api/skills/install-starter");
    await page.addInitScript(() => {
      localStorage.setItem("anycode-dashboard-locale", "zh");
    });
    await page.goto("/agents");
    await expect(page.locator("#agents-tab-installed")).toBeVisible({ timeout: 15_000 });
    await page.locator("#agents-tab-installed").click();
    await expect(page.locator("#agents-panel-installed")).toBeVisible();
  });

  test("skills market API returns entries", async ({ request }) => {
    const res = await request.get("/api/skills/market");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.market?.entries)).toBeTruthy();
  });
});
