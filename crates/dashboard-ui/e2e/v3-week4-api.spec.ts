import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 4 API", () => {
  test("session usage endpoint shape", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=5");
    expect(sessions.ok()).toBeTruthy();
    const body = await sessions.json();
    const list = body.sessions ?? [];
    if (list.length === 0) {
      test.skip(true, "no sessions");
    }
    const sid = list[0].id as string;
    const usage = await request.get(`/api/sessions/${sid}/usage`);
    expect(usage.ok()).toBeTruthy();
    const u = await usage.json();
    expect(typeof u.usage.total_tokens).toBe("number");
    expect(Array.isArray(u.by_model)).toBeTruthy();
  });

  test("gate stream endpoint accepts POST", async ({ request }) => {
    const projects = await request.get("/api/projects");
    const list = (await projects.json()).projects ?? [];
    if (list.length === 0) test.skip(true, "no projects");
    const pid = list[0].id as string;
    const res = await request.post(`/api/projects/${pid}/gates/execute/stream`, {
      data: { command: "echo STREAM_OK", name: "echo" },
      headers: { Accept: "text/event-stream" },
    });
    expect(res.ok()).toBeTruthy();
    const text = await res.text();
    expect(text.includes("line") || text.includes("done") || text.includes("STREAM")).toBeTruthy();
  });
});
