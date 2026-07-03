import { expect, test } from "@playwright/test";

test.describe("Settings gates", () => {
  test("gate prefs persist via API", async ({ request }) => {
    const get1 = await request.get("/api/settings/gate-prefs");
    expect(get1.ok()).toBeTruthy();
    const before = await get1.json();

    const put = await request.put("/api/settings/gate-prefs", {
      data: {
        acceptance_gates_default: !before.acceptance_gates_default,
        default_acceptance_preset_ids: before.default_acceptance_preset_ids ?? [],
      },
    });
    expect(put.ok()).toBeTruthy();

    const get2 = await request.get("/api/settings/gate-prefs");
    const after = await get2.json();
    expect(after.acceptance_gates_default).toBe(!before.acceptance_gates_default);

    await request.put("/api/settings/gate-prefs", { data: before });
  });

  test("gates settings section loads", async ({ page }) => {
    await page.goto("/settings?section=gates");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByText(/acceptance|验收/i).first()).toBeVisible();
  });
});
