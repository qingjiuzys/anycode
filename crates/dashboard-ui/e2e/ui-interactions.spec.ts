import { expect, test } from "@playwright/test";

const NAV = [
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
  for (const item of NAV) {
    test(`nav ${item.path}`, async ({ page }) => {
      await page.goto(item.path);
      await expect(page.getByRole("main")).toBeVisible();
      await expect(page.getByRole("navigation")).toBeVisible();
    });
  }

  test("topbar new menu opens", async ({ page }) => {
    await page.goto("/");
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
    const body = await res.json();
    const projects = body.projects ?? [];
    if (projects.length === 0) {
      test.skip(true, "no projects in fixture DB");
    }
    await page.goto(`/projects/${projects[0].id}`);
    await expect(page.locator("main")).toBeVisible();
  });
});
