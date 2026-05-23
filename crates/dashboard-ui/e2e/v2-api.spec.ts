import { expect, test } from "@playwright/test";

/**
 * V2 API smoke — usage export, project usage, gate presets, GitHub connector shape.
 */
test.describe("Digital Workbench V2 API", () => {
  test("usage export returns CSV", async ({ request }) => {
    const res = await request.get("/api/metrics/usage/export?days=7");
    expect(res.ok()).toBeTruthy();
    const text = await res.text();
    expect(text.startsWith("project_id,project_name,")).toBeTruthy();
  });

  test("project usage and gate presets", async ({ request }) => {
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
    expect(typeof u.usage.total_tokens).toBe("number");

    const presets = await request.get(`/api/projects/${pid}/gates/presets`);
    expect(presets.ok()).toBeTruthy();
    const p = await presets.json();
    expect(Array.isArray(p.presets)).toBeTruthy();
  });

  test("github connector issues endpoint shape", async ({ request }) => {
    const conn = await request.post("/api/settings/connectors", {
      data: {
        source_type: "github",
        name: "e2e-github",
        config: { repo: "not-a-repo" },
        enabled: true,
      },
    });
    expect(conn.ok()).toBeTruthy();
    const c = await conn.json();
    const id = c.connector?.id as string;
    expect(id).toBeTruthy();

    const issues = await request.get(`/api/settings/connectors/${id}/github/issues`);
    expect(issues.status()).toBe(502);
    const err = await issues.json();
    expect(err.error).toBeTruthy();
  });
});
