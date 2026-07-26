import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 3 API", () => {
  test("cancel on a terminal session is idempotent (already_idle)", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=5");
    expect(sessions.ok()).toBeTruthy();
    const body = await sessions.json();
    const list = body.sessions ?? [];
    const completed = list.find((s: { status: string }) => s.status !== "running");
    if (!completed) {
      test.skip(true, "no non-running session");
    }
    const res = await request.post(`/api/sessions/${completed.id}/cancel`);
    // Contract: cancelling an already-terminal session succeeds idempotently
    // so the client can refresh and unstick — not a 409 conflict.
    expect(res.status()).toBe(200);
    const payload = await res.json();
    expect(payload.ok).toBe(true);
    expect(payload.already_idle).toBe(true);
  });
});
