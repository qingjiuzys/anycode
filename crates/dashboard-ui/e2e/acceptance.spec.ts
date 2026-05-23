import { expect, test } from "@playwright/test";

/**
 * V1 UX acceptance (7 items) — smoke against live dashboard API + UI.
 */
test.describe("Digital Workbench UX acceptance", () => {
  test("1 — dashboard serves UI and health API", async ({ page, request }) => {
    const health = await request.get("/api/health");
    expect(health.ok()).toBeTruthy();
    await page.goto("/");
    await expect(page.locator("body")).toBeVisible();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  });

  test("2 — select a project", async ({ page }) => {
    await page.goto("/projects");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.locator("main")).toBeVisible();
  });

  test("3 — project detail shows sessions region", async ({ page, request }) => {
    const res = await request.get("/api/projects");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const projects = body.projects ?? [];
    if (projects.length === 0) {
      test.skip(true, "no projects in fixture DB");
    }
    await page.goto(`/projects/${projects[0].id}`);
    await expect(page.locator("main")).toBeVisible();
  });

  test("4 — session detail / replay API shape", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=5");
    expect(sessions.ok()).toBeTruthy();
    const body = await sessions.json();
    const list = body.sessions ?? [];
    if (list.length === 0) {
      test.skip(true, "no sessions");
    }
    const replay = await request.get(`/api/sessions/${list[0].id}/replay`);
    expect(replay.ok()).toBeTruthy();
    const r = await replay.json();
    expect(r.replay).toBeTruthy();
    expect(r.replay.status).toBeTruthy();
  });

  test("5 — settings trust / doctor reachable", async ({ page, request }) => {
    const doctor = await request.get("/api/settings/doctor");
    expect(doctor.ok()).toBeTruthy();
    await page.goto("/settings");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  });

  test("6 — assets page loads", async ({ page }) => {
    await page.goto("/assets");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  });

  test("7 — configure port + SQLite preferences", async ({ page, request }) => {
    const prefs = await request.get("/api/settings/preferences");
    expect(prefs.ok()).toBeTruthy();
    const body = await prefs.json();
    expect(body.preferences?.active?.port).toBeTruthy();
    await page.goto("/settings");
    await expect(page.locator("main")).toContainText(/43180|port|Port|端口/i);
  });
});

test("metrics usage API", async ({ request }) => {
  const res = await request.get("/api/metrics/usage?days=7");
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  expect(body.usage).toBeTruthy();
  expect(typeof body.usage.total_tokens).toBe("number");
});
