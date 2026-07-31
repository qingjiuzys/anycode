import { expect, test } from "@playwright/test";

test.describe("runtime alignment smoke", () => {
  test("prompt preview has no duplicate channel_specific segment", async ({ request }) => {
    const res = await request.get("/api/settings/prompt-preview?agent=general-purpose");
    expect(res.ok()).toBeTruthy();
    const body = (await res.json()) as {
      segments?: Array<{ id: string }>;
    };
    const ids = (body.segments ?? []).map((s) => s.id);
    expect(ids).not.toContain("channel_specific");
    expect(ids).toContain("default_stack");
  });

  test("unknown skill id rejected on conversation start", async ({ request }) => {
    const projects = await request.get("/api/projects?limit=5");
    expect(projects.ok()).toBeTruthy();
    const plist = (await projects.json()) as { projects?: Array<{ id: string }> };
    const projectId = plist.projects?.[0]?.id;
    expect(projectId).toBeTruthy();
    const res = await request.post(`/api/projects/${projectId}/conversations/start`, {
      data: {
        prompt: "hello",
        kind: "run",
        skills: ["definitely-not-a-real-skill-id-xyz"],
      },
    });
    expect(res.status()).toBe(400);
    const body = (await res.json()) as { error?: string };
    expect(body.error ?? "").toMatch(/unknown skill/i);
  });

  test("completed session fixture exposes terminal status", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=20");
    expect(sessions.ok()).toBeTruthy();
    const body = (await sessions.json()) as {
      sessions?: Array<{ id: string; status?: string }>;
    };
    const completed = (body.sessions ?? []).find((s) => s.status === "completed");
    expect(completed, "fixture should include a completed session").toBeTruthy();
  });
});
