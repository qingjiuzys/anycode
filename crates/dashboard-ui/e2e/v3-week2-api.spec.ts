import { expect, test } from "@playwright/test";

/**
 * V3 Week 2 API — Linear connector shape.
 */
test.describe("Digital Workbench V3 Week 2 API", () => {
  test("linear connector rejects missing API key on create", async ({ request }) => {
    const conn = await request.post("/api/settings/connectors", {
      data: {
        source_type: "linear",
        name: "e2e-linear-invalid",
        config: { team_key: "ENG" },
        enabled: true,
      },
    });
    expect(conn.status()).toBe(400);
    const err = await conn.json();
    expect(String(err.error ?? "")).toMatch(/API key/i);
  });

  test("linear connector issues endpoint shape", async ({ request }) => {
    let id: string | undefined;
    try {
      const conn = await request.post("/api/settings/connectors", {
        data: {
          source_type: "linear",
          name: "e2e-linear",
          config: { team_key: "ENG", token: "e2e-fake-token" },
          enabled: true,
        },
      });
      expect(conn.ok()).toBeTruthy();
      const c = await conn.json();
      id = c.connector?.id as string;
      expect(id).toBeTruthy();

      const issues = await request.get(`/api/settings/connectors/${id}/linear/issues`);
      // Token is redacted on store; without LINEAR_API_KEY env this should 502.
      expect(issues.status()).toBe(502);
      const err = await issues.json();
      expect(err.error).toBeTruthy();
    } finally {
      if (id) {
        await request.delete(`/api/settings/connectors/${id}`);
      }
    }
  });
});
