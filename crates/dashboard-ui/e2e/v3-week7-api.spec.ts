import { expect, test } from "@playwright/test";

test.describe("Digital Workbench V3 Week 7 API", () => {
  test("trigger run validates empty prompt", async ({ request }) => {
    const projects = await request.get("/api/projects");
    const list = (await projects.json()).projects ?? [];
    if (list.length === 0) test.skip(true, "no projects");
    const pid = list[0].id as string;
    const res = await request.post(`/api/projects/${pid}/runs/trigger`, {
      data: { prompt: "   ", kind: "run" },
    });
    expect(res.status()).toBe(400);
  });

  test("list triggers endpoint shape", async ({ request }) => {
    const projects = await request.get("/api/projects");
    const list = (await projects.json()).projects ?? [];
    if (list.length === 0) test.skip(true, "no projects");
    const pid = list[0].id as string;
    const res = await request.get(`/api/projects/${pid}/runs/triggers?limit=5`);
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(Array.isArray(body.triggers)).toBeTruthy();
  });
});
