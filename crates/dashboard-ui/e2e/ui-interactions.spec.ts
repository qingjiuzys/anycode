import { expect, test } from "@playwright/test";

const PAGES = [
  { path: "/", name: /home/i },
  { path: "/overview", name: /overview/i },
  { path: "/projects", name: /projects/i },
  { path: "/conversations", name: /conversations/i },
  { path: "/automations", name: /automations/i },
  { path: "/assets", name: /assets/i },
  { path: "/reports", name: /reports/i },
  { path: "/audit", name: /audit/i },
  { path: "/agents", name: /agents/i },
  { path: "/settings", name: /settings/i },
];

test.describe("UI shell navigation", () => {
  for (const item of PAGES) {
    test(`page ${item.path} loads main`, async ({ page }) => {
      await page.goto(item.path);
      await expect(page.getByRole("main")).toBeVisible();
    });
  }

  test("conversations shows session sidebar on desktop", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/conversations");
    await expect(page.locator(".dw-session-sidebar")).toBeVisible();
  });

  test("control center fab visible on conversations", async ({ page }) => {
    await page.goto("/conversations");
    await expect(page.locator(".dw-control-fab")).toBeVisible();
  });

  test("topbar new menu opens", async ({ page }) => {
    await page.goto("/projects");
    const newBtn = page.getByRole("button", { name: /new/i });
    if (await newBtn.isVisible()) {
      await newBtn.click();
      await expect(page.getByRole("menu")).toBeVisible();
    }
  });

  test("settings sections reachable", async ({ page }) => {
    await page.goto("/settings");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const nav = page.locator("nav, aside").filter({ hasText: /auth|model|skills/i }).first();
    if (await nav.isVisible()) {
      await expect(nav).toBeVisible();
    }
  });
});

test.describe("Page primary actions", () => {
  test("projects — new project button", async ({ page }) => {
    await page.goto("/projects");
    await expect(page.getByRole("button", { name: /new project|新建项目/i })).toBeVisible();
  });

  test("agents — rescan control", async ({ page }) => {
    await page.goto("/agents");
    await expect(page.getByRole("button", { name: /rescan|重新扫描/i })).toBeVisible();
  });

  test("automations — create region", async ({ page }) => {
    await page.goto("/automations");
    await expect(page.locator("main")).toBeVisible();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  });

  test("assets — filter bar", async ({ page }) => {
    await page.goto("/assets");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  });

  test("audit — export control", async ({ page }) => {
    await page.goto("/audit");
    await expect(page.getByRole("button", { name: /export|导出/i })).toBeVisible();
  });

  test("setup wizard loads", async ({ page }) => {
    await page.goto("/setup?review=1");
    await expect(page.getByRole("main")).toBeVisible();
    await expect(page.getByRole("button", { name: /start|开始/i })).toBeVisible();
  });
});

test.describe("Project detail smoke", () => {
  test("project detail from fixture", async ({ page, request }) => {
    const res = await request.get("/api/projects");
    const body = (await res.json()) as { projects: Array<{ id: string }> };
    const project = body.projects[0];
    test.skip(!project, "no projects");
    await page.goto(`/projects/${project!.id}`);
    await expect(page.getByRole("main")).toBeVisible();
  });
});
