import { expect, test } from "@playwright/test";

const LIVE = process.env.LIVE_E2E === "1";

test.describe("live cloud conversation smoke", () => {
  test.skip(!LIVE, "set LIVE_E2E=1 to run real LLM conversation smoke");

  test("sends prompt and receives assistant reply without missing_turn", async ({
    page,
    request,
  }) => {
    const health = await request.get("/api/health");
    expect(health.ok()).toBeTruthy();

    const gw = await request.post("/api/cloud/gateway-test");
    expect(gw.ok()).toBeTruthy();
    const gwBody = (await gw.json()) as { ok?: boolean };
    expect(gwBody.ok).toBe(true);

    await page.goto("/conversations");
    await expect(page.locator(".dw-composer")).toBeVisible({ timeout: 15_000 });

    const textarea = page.locator(".dw-composer textarea, .dw-composer [contenteditable='true']").first();
    await textarea.click();
    await textarea.fill("Reply with exactly the word pong, nothing else.");

    const send = page.locator(
      ".dw-composer button[type='submit'], .dw-composer .dw-send, .dw-composer button:has-text('Send'), .dw-composer button:has-text('发送')",
    ).first();
    await send.click();

    await expect(
      page.locator(".bubble-assistant-live, .bubble-assistant").filter({ hasText: /pong/i }),
    ).toBeVisible({ timeout: 120_000 });

    await expect(page.locator("text=missing_turn")).toHaveCount(0);
  });
});
