import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 9 API", () => {
  test("approval summary endpoint shape", async ({ request }) => {
    const res = await request.get("/api/security/approvals/summary");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(typeof body.summary.pending_total).toBe("number");
    expect(Array.isArray(body.summary.by_session)).toBeTruthy();
    expect(typeof body.web_enabled).toBe("boolean");
  });

  test("pending approvals accepts session_id filter", async ({ request }) => {
    const res = await request.get(
      "/api/security/approvals/pending?limit=5&session_id=sess_missing",
    );
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.pending)).toBeTruthy();
    expect(body.pending.length).toBe(0);
  });
});
