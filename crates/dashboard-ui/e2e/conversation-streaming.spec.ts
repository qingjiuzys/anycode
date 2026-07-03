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
});
