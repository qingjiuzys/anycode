import { expect, test } from "@playwright/test";

test.describe("control center", () => {
  test("opens from fab and closes with escape", async ({ page }) => {
    await page.goto("/conversations");
    const fab = page.locator(".dw-control-fab");
    await expect(fab).toBeVisible({ timeout: 10_000 });
    await fab.click();
    await expect(page.locator(".dw-control-center")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator(".dw-control-center")).toHaveCount(0);
    await expect(fab).toBeVisible();
  });

  test("back button closes overlay", async ({ page }) => {
    await page.goto("/conversations");
    await page.locator(".dw-control-fab").click();
    await page.getByRole("button", { name: /back|返回/i }).click();
    await expect(page.locator(".dw-control-center")).toHaveCount(0);
  });

  test("switches embedded settings panel", async ({ page }) => {
    await page.goto("/conversations");
    await page.locator(".dw-control-fab").click();
    await page.getByRole("button", { name: /settings|设置/i }).click();
    await expect(page.locator(".dw-settings-nav")).toBeVisible();
  });

  test("deep-link opens overlay on conversations", async ({ page }) => {
    await page.goto("/settings");
    await expect(page).toHaveURL(/\/conversations/);
    await expect(page.locator(".dw-control-center")).toBeVisible();
    await expect(page.locator(".dw-settings-nav")).toBeVisible();
  });

  test("nested project detail stays in overlay", async ({ page }) => {
    await page.goto("/conversations");
    await page.locator(".dw-control-fab").click();
    await page.getByRole("button", { name: /projects|项目/i }).click();
    const projectLink = page.locator("table tbody tr a, table tbody tr button").first();
    await expect(projectLink).toBeVisible({ timeout: 10_000 });
    await projectLink.click();
    await expect(page.locator(".dw-control-center")).toBeVisible();
    await expect(page).toHaveURL(/\/conversations/);
  });
});

test.describe("session routes", () => {
  test("session detail without tab redirects to conversations", async ({ page, request }) => {
    const sessions = await request.get("/api/sessions?limit=1");
    test.skip(!sessions.ok(), "api unavailable");
    const body = (await sessions.json()) as {
      sessions?: Array<{ id: string }>;
    };
    const sid = body.sessions?.[0]?.id;
    test.skip(!sid, "no sessions");
    await page.goto(`/sessions/${sid}`);
    await expect(page).toHaveURL(/\/conversations\?.*session=/);
  });
});
