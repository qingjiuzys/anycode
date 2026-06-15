import { expect, test } from "@playwright/test";

test.describe("workbench API", () => {
  test("fs list for first project", async ({ request }) => {
    const projects = await request.get("/api/projects?limit=5");
    expect(projects.ok()).toBeTruthy();
    const body = (await projects.json()) as { projects: Array<{ id: string; root_path: string }> };
    const project = body.projects[0];
    test.skip(!project, "no projects registered");

    const list = await request.get(
      `/api/projects/${encodeURIComponent(project!.id)}/fs/list?path=`,
    );
    expect(list.ok()).toBeTruthy();
    const fs = (await list.json()) as { entries: unknown[] };
    expect(Array.isArray(fs.entries)).toBeTruthy();
  });
});

test.describe("workbench sidebar UI", () => {
  test("conversations page shows workbench rail when session selected", async ({ page }) => {
    await page.goto("/conversations");
    const sessionItem = page.locator("[class*='ConversationSessionList'], .dw-main").locator("button, a").filter({ hasText: /.+/ }).first();
    if (!(await sessionItem.isVisible({ timeout: 5000 }).catch(() => false))) {
      test.skip(true, "no sessions");
    }
    await sessionItem.click();
    const rail = page.locator(".conv-workbench-rail");
    await expect(rail).toBeVisible({ timeout: 10_000 });
    await rail.locator("button").first().click();
    await expect(page.locator(".conv-workbench-panel")).toBeVisible();
  });
});
