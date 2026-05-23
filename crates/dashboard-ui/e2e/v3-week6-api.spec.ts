import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 6 API", () => {
  test("cancel session response includes live_signal", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=5");
    const list = (await sessions.json()).sessions ?? [];
    if (list.length === 0) test.skip(true, "no sessions");
    const sid = list[0].id as string;
    const res = await request.post(`/api/sessions/${sid}/cancel`, { data: {} });
    expect([200, 409]).toContain(res.status());
    if (res.ok()) {
      const body = await res.json();
      expect(typeof body.live_signal).toBe("boolean");
    }
  });
});
