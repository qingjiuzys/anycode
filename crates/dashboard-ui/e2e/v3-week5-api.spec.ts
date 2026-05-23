import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 5 API", () => {
  test("security activity endpoint shape", async ({ request }) => {
    const res = await request.get("/api/security/activity?limit=5");
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const summary = body.summary;
    expect(typeof summary.denied_total).toBe("number");
    expect(typeof summary.pending_total).toBe("number");
    expect(Array.isArray(summary.recent)).toBeTruthy();
    expect(typeof summary.read_only).toBe("boolean");
    expect(typeof summary.note).toBe("string");
  });
});
