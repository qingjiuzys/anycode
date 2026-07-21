import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test.describe("deliverable fs/raw", () => {
  test("streams binary under project root", async ({ request }) => {
    const projects = await request.get("/api/projects?limit=5");
    expect(projects.ok()).toBeTruthy();
    const body = (await projects.json()) as {
      projects: Array<{ id: string; root_path: string }>;
    };
    const project = body.projects[0];
    test.skip(!project, "no projects registered");

    const rel = `.anycode-e2e-deliverable-${Date.now()}.png`;
    // 1x1 PNG
    const png = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
      "base64",
    );
    const abs = path.join(project!.root_path, rel);
    fs.writeFileSync(abs, png);
    try {
      const raw = await request.get(
        `/api/projects/${encodeURIComponent(project!.id)}/fs/raw?path=${encodeURIComponent(rel)}`,
      );
      expect(raw.ok()).toBeTruthy();
      const ct = raw.headers()["content-type"] ?? "";
      expect(ct.includes("image") || ct.includes("octet-stream")).toBeTruthy();
      const buf = Buffer.from(await raw.body());
      expect(buf.length).toBe(png.length);
    } finally {
      fs.unlinkSync(abs);
    }
  });
});

test.describe("deliverable card smoke (UI)", () => {
  test("artifacts panel loads for a session", async ({ page }) => {
    await page.goto("/conversations");
    const sessionItem = page
      .locator(".dw-main")
      .locator("button, a")
      .filter({ hasText: /.+/ })
      .first();
    if (!(await sessionItem.isVisible({ timeout: 5000 }).catch(() => false))) {
      test.skip(true, "no sessions");
    }
    await sessionItem.click();
    const rail = page.locator(".conv-workbench-rail");
    await expect(rail).toBeVisible({ timeout: 10_000 });
    const artifactsTab = rail.locator("button").filter({ hasText: /产物|Artifacts|交付/i }).first();
    if (await artifactsTab.isVisible({ timeout: 3000 }).catch(() => false)) {
      await artifactsTab.click();
    }
    await expect(page.locator(".conv-workbench-panel")).toBeVisible();
  });
});
