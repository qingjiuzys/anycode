import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 8 API", () => {
  test("pending approvals endpoint shape", async ({ request }) => {
    const res = await request.get("/api/security/approvals/pending?limit=5");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.pending)).toBeTruthy();
    expect(typeof body.web_enabled).toBe("boolean");
    expect(typeof body.respond_allowed).toBe("boolean");
  });

  test("respond rejects unknown approval", async ({ request }) => {
    const res = await request.post("/api/security/approvals/apr_missing/respond", {
      data: { decision: "deny" },
    });
    expect(res.status()).toBe(404);
  });
});
