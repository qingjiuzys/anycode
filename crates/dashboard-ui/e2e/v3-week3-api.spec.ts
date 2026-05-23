import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 3 API", () => {
  test("cancel session returns conflict when not running", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=5");
    expect(sessions.ok()).toBeTruthy();
    const body = await sessions.json();
    const list = body.sessions ?? [];
    const completed = list.find((s: { status: string }) => s.status !== "running");
    if (!completed) {
      test.skip(true, "no non-running session");
    }
    const res = await request.post(`/api/sessions/${completed.id}/cancel`);
    expect(res.status()).toBe(409);
  });
});
