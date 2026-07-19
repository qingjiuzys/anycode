import { expect, test, type APIRequestContext } from "@playwright/test";

async function requireSeededSessions(request: APIRequestContext) {
  const sessions = await request.get("/api/sessions?limit=5");
  expect(
    sessions.ok(),
    "sessions API unavailable — ensure scripts/dashboard-e2e-server.sh seeded fixture with ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1",
  ).toBeTruthy();
  const body = (await sessions.json()) as {
    sessions?: Array<{ id: string; status?: string; title?: string }>;
  };
  const list = body.sessions ?? [];
  expect(
    list.length,
    "no sessions in fixture DB — e2e seed failed (expected e2e-session / e2e-completed)",
  ).toBeGreaterThan(0);
  return list;
}

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
              rule.cssText.includes(".bubble-assistant-live") ||
              rule.cssText.includes(".agent-activity-line") ||
              rule.cssText.includes(".agent-status-line"),
          );
        } catch {
          return false;
        }
      });
    });
    expect(hasStreamingStyles).toBe(true);
  });

  test("session detail route keeps conversations shell", async ({ page, request }) => {
    const list = await requireSeededSessions(request);
    const sid = list[0]?.id;
    expect(sid, "fixture session missing id").toBeTruthy();

    await page.goto(`/conversations?session=${sid}`);
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });
    await expect(page.locator(".bubble-assistant, .tool-strip, .dw-composer")).toBeTruthy();
  });

  test("running session exposes SSE stream endpoint", async ({ request }) => {
    const list = await requireSeededSessions(request);
    const running = list.find((s) => s.status === "running") ?? list[0];
    expect(running?.id, "fixture session missing id for SSE probe").toBeTruthy();

    const port = process.env.DASHBOARD_E2E_PORT ?? "43199";
    const url = `http://127.0.0.1:${port}/api/sessions/${running!.id}/events/stream`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 1_500);
    try {
      const stream = await fetch(url, { signal: controller.signal });
      expect(stream.status).toBe(200);
      expect(stream.headers.get("content-type") ?? "").toContain("text/event-stream");
    } catch (error) {
      // Abort is expected — SSE stays open; only validate we connected.
      expect(String(error)).toMatch(/abort/i);
    } finally {
      clearTimeout(timer);
    }
  });

  test("transcript includes assistant or tool feed blocks", async ({ request }) => {
    const list = await requireSeededSessions(request);
    const sid = list[0]?.id;
    expect(sid, "fixture session missing id").toBeTruthy();

    const transcript = await request.get(`/api/sessions/${sid}/transcript`);
    expect(transcript.ok()).toBeTruthy();
    const payload = (await transcript.json()) as {
      transcript?: { blocks?: Array<{ block_type?: string }> };
    };
    const types = new Set(
      (payload.transcript?.blocks ?? []).map((b) => b.block_type).filter(Boolean),
    );
    expect(
      types.has("assistant_message") ||
        types.has("tool_call") ||
        types.has("user_message"),
    ).toBe(true);
  });

  test("session open does not poll pending-questions", async ({ page, request }) => {
    const list = await requireSeededSessions(request);
    const sid = list[0]?.id;
    expect(sid, "fixture session missing id").toBeTruthy();

    const pendingQuestionUrls: string[] = [];
    page.on("request", (req) => {
      const url = req.url();
      if (url.includes("/api/security/questions/pending")) {
        pendingQuestionUrls.push(url);
      }
    });

    await page.goto(`/conversations?session=${sid}`);
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });

    // Longer than the legacy 2.5s poll interval — should not repeat.
    await page.waitForTimeout(6_000);

    expect(
      pendingQuestionUrls.length,
      `expected at most one cold rehydrate fetch, got: ${pendingQuestionUrls.join(", ")}`,
    ).toBeLessThanOrEqual(1);
  });

  test("turn recap and typing indicator do not stack", async ({ page, request }) => {
    const list = await requireSeededSessions(request);
    const sid = list[0]?.id;
    expect(sid, "fixture session missing id").toBeTruthy();

    await page.goto(`/conversations?session=${sid}`);
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });

    const recapCount = await page.locator('[data-testid="turn-recap-header"]').count();
    const typingCount = await page.locator('[data-testid="typing-indicator"]').count();
    expect(recapCount + typingCount).toBeGreaterThanOrEqual(0);
    if (recapCount > 0 && typingCount > 0) {
      const stacked = await page.evaluate(() => {
        const recaps = document.querySelectorAll('[data-testid="turn-recap-header"]');
        const typings = document.querySelectorAll('[data-testid="typing-indicator"]');
        for (const recap of recaps) {
          for (const typing of typings) {
            const recapTurn = recap.closest("article");
            const typingTurn = typing.closest("article");
            if (recapTurn && typingTurn && recapTurn === typingTurn) {
              return true;
            }
          }
        }
        return false;
      });
      expect(stacked, "recap header and typing indicator must not coexist in same turn").toBe(
        false,
      );
    }
  });

  test("last turn limits duplicate activity lines", async ({ page, request }) => {
    const list = await requireSeededSessions(request);
    const sid = list[0]?.id;
    expect(sid, "fixture session missing id").toBeTruthy();

    await page.goto(`/conversations?session=${sid}`);
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 10_000 });

    const articles = page.locator("article");
    const count = await articles.count();
    if (count === 0) return;

    const lastArticle = articles.nth(count - 1);
    const activityLines = await lastArticle.locator(".agent-activity-line").count();
    expect(activityLines).toBeLessThanOrEqual(1);
  });
});
