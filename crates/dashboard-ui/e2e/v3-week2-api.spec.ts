import { expect, test } from "@playwright/test";

/**
 * V3 Week 2 API — Linear connector shape.
 */
test.describe("Digital Workbench V3 Week 2 API", () => {
  test("linear connector issues endpoint shape", async ({ request }) => {
    const conn = await request.post("/api/settings/connectors", {
      data: {
        source_type: "linear",
        name: "e2e-linear",
        config: { team_key: "ENG" },
        enabled: true,
      },
    });
    expect(conn.ok()).toBeTruthy();
    const c = await conn.json();
    const id = c.connector?.id as string;
    expect(id).toBeTruthy();

    const issues = await request.get(`/api/settings/connectors/${id}/linear/issues`);
    expect(issues.status()).toBe(502);
    const err = await issues.json();
    expect(err.error).toBeTruthy();
  });
});
