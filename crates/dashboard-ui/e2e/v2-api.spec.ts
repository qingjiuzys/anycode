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

  test("github connector rejects invalid repo on create", async ({ request }) => {
    const bad = await request.post("/api/settings/connectors", {
      data: {
        source_type: "github",
        name: "e2e-github-invalid",
        config: { repo: "not-a-repo" },
        enabled: true,
      },
    });
    expect(bad.status()).toBe(400);
    const err = await bad.json();
    expect(String(err.error ?? "")).toMatch(/owner\/repo/i);
  });

  test("github connector issues endpoint shape", async ({ request }) => {
    let id: string | undefined;
    try {
      const conn = await request.post("/api/settings/connectors", {
        data: {
          source_type: "github",
          name: "e2e-github",
          config: { repo: "octocat/Hello-World" },
          enabled: true,
        },
      });
      expect(conn.ok()).toBeTruthy();
      const c = await conn.json();
      id = c.connector?.id as string;
      expect(id).toBeTruthy();

      const issues = await request.get(`/api/settings/connectors/${id}/github/issues`);
      // Public repo may succeed (200) or fail with gateway/timeout (502).
      expect([200, 502]).toContain(issues.status());
      const body = await issues.json();
      if (issues.status() === 200) {
        expect(Array.isArray(body.issues)).toBeTruthy();
      } else {
        expect(body.error).toBeTruthy();
      }
      // Create shape is the primary contract; live GitHub is best-effort.
    } finally {
      if (id) {
        await request.delete(`/api/settings/connectors/${id}`);
      }
    }
  });
});
