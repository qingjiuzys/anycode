import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 10 API", () => {
  test("bootstrap reports v3_week10 phase", async ({ request }) => {
    const res = await request.get("/api/bootstrap");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.bootstrap.workbench_phase).toBe("v3_week10");
  });

  test("approval summary still powers conversations filter", async ({ request }) => {
    const res = await request.get("/api/security/approvals/summary");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(typeof body.summary.pending_total).toBe("number");
  });
});
