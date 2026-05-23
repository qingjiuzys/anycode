import { expect, test } from "@playwright/test";

/**
 * V3 API smoke — per-model usage breakdown and saved-hours KPI.
 */
test.describe("Digital Workbench V3 API", () => {
  test("usage metrics include by_model", async ({ request }) => {
    const res = await request.get("/api/metrics/usage?days=7");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(typeof body.usage.total_tokens).toBe("number");
    expect(Array.isArray(body.by_model)).toBeTruthy();
  });

  test("saved-hours KPI shape", async ({ request }) => {
    const res = await request.get("/api/metrics/kpi/saved-hours?days=7");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(typeof body.kpi.estimated_saved_hours).toBe("number");
    expect(typeof body.kpi.estimated_value_usd).toBe("number");
    expect(typeof body.kpi.sessions_completed).toBe("number");
  });

  test("project usage includes by_model", async ({ request }) => {
    const projects = await request.get("/api/projects");
    expect(projects.ok()).toBeTruthy();
    const body = await projects.json();
    const list = body.projects ?? [];
    if (list.length === 0) {
      test.skip(true, "no projects");
    }
    const pid = list[0].id as string;
    const usage = await request.get(`/api/projects/${pid}/usage?days=7`);
    expect(usage.ok()).toBeTruthy();
    const u = await usage.json();
    expect(Array.isArray(u.by_model)).toBeTruthy();
  });
});
