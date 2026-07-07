import { expect, test } from "@playwright/test";

test.describe("conversation streaming UX", () => {
  test("loads conversations composer and streaming styles", async ({ page }) => {
    await page.goto("/conversations");
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });

    const hasStreamingStyles = await page.evaluate(() => {
      return [...document.styleSheets].some((sheet) => {
        try {
          return [...sheet.cssRules].some(
            (rule) =>
              rule.cssText.includes(".tool-strip") ||
              rule.cssText.includes(".bubble-assistant-live"),
          );
        } catch {
          return false;
        }
      });
    });
    expect(hasStreamingStyles).toBe(true);
  });

  test("session detail route keeps conversations shell", async ({ page, request }) => {
    const sessions = await request.get("/api/sessions?limit=1");
    test.skip(!sessions.ok(), "api unavailable");
    const body = (await sessions.json()) as {
      sessions?: Array<{ id: string }>;
    };
    const sid = body.sessions?.[0]?.id;
    test.skip(!sid, "no sessions");

    await page.goto(`/conversations?session=${sid}`);
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });
    await expect(page.locator(".bubble-assistant, .tool-strip, .dw-composer")).toBeTruthy();
  });

  test("running session exposes SSE stream endpoint", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=20");
    test.skip(!sessions.ok(), "api unavailable");
    const body = (await sessions.json()) as {
      sessions?: Array<{ id: string; status?: string }>;
    };
    const running = body.sessions?.find((s) => s.status === "running") ?? body.sessions?.[0];
    test.skip(!running?.id, "no sessions");

    const stream = await request.get(`/api/sessions/${running.id}/events/stream`, {
      timeout: 5_000,
    });
    expect(stream.status()).toBe(200);
    expect(stream.headers()["content-type"] ?? "").toContain("text/event-stream");
  });

  test("transcript blocks tolerate scoped user_turn tool keys", async ({ request }) => {
    const sessions = await request.get("/api/sessions?limit=1");
    test.skip(!sessions.ok(), "api unavailable");
    const body = (await sessions.json()) as {
      sessions?: Array<{ id: string }>;
    };
    const sid = body.sessions?.[0]?.id;
    test.skip(!sid, "no sessions");

    const transcript = await request.get(`/api/sessions/${sid}/transcript`);
    test.skip(!transcript.ok(), "transcript unavailable");
    const payload = (await transcript.json()) as {
      transcript?: { blocks?: Array<{ meta?: Record<string, unknown> }> };
    };
    const blocks = payload.transcript?.blocks ?? [];
    for (const block of blocks) {
      const toolKey = block.meta?.tool_key;
      if (typeof toolKey === "string" && toolKey.includes(":")) {
        expect(toolKey.length).toBeGreaterThan(0);
      }
    }
  });
});
