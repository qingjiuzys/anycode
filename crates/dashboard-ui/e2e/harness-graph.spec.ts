import { spawnSync } from "node:child_process";
import path from "node:path";
import { expect, test } from "@playwright/test";

const APPLE_MEDIA_HELPER = path.resolve(
  import.meta.dirname,
  "../../../apps/anycode-desktop/resources/bin/anycode-apple-media",
);

function helperOp(request: Record<string, unknown>): { ok?: boolean; text?: string; error?: string } {
  const ran = spawnSync(APPLE_MEDIA_HELPER, [], {
    input: JSON.stringify(request),
    encoding: "utf8",
  });
  expect(ran.status, ran.stderr).toBe(0);
  return JSON.parse(ran.stdout.split("\n")[0] ?? "{}") as {
    ok?: boolean;
    text?: string;
    error?: string;
  };
}

test.describe("Harness graph preview", () => {
  test("status stays unarmed and Start/Resume stay disabled", async ({ page, request }) => {
    const status = await request.get("/api/harness/status");
    expect(status.ok()).toBeTruthy();
    const json = await status.json();
    expect(json.graph).toBe(false);
    expect(json.unified_kernel).toBe(false);
    expect(json.computer).toBe(false);
    expect(json.x11_verified).toBe(false);

    await page.goto("/harness/graph");
    await expect(page.getByRole("heading", { name: "Harness Graph", exact: true })).toBeVisible();
    await expect(
      page.getByText("Preview only. Arm harness-v1-graph and open a project before Start/Resume enable."),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Start validated graph" })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Resume explicitly" })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Pair this desktop" })).toHaveCount(0);
  });

  test("Pair this desktop confirms on the BFF and invokes the keychain command", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
      Object.defineProperty(window, "__TAURI_PAIRING_CALLS__", {
        configurable: true,
        get() {
          return calls;
        },
      });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        configurable: true,
        value: {
          invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
            calls.push({ cmd, args });
            if (cmd === "harness_pairing_origin_configured") {
              return { configured: true };
            }
            if (cmd === "harness_pairing_post") {
              const path = String(args.path ?? "");
              if (path.endsWith("/sso/begin")) {
                return {
                  authorize_url: "https://example.invalid/hop",
                  poll_token: "B".repeat(43),
                };
              }
              if (path.endsWith("/sso/poll")) {
                return { access_token: "C".repeat(43) };
              }
              const headers: Record<string, string> = {
                "content-type": "application/json",
              };
              const bearer = typeof args.bearer === "string" ? args.bearer.trim() : "";
              // Chromium always sends Origin; Desktop reqwest does not.
              // Pairing routes skip the allowlist only when a Bearer is present.
              headers.authorization = `Bearer ${bearer || "playwright-desktop-pairing"}`;
              const res = await fetch(path, {
                method: "POST",
                headers,
                body: JSON.stringify(args.body ?? {}),
              });
              const json = await res.json();
              if (!res.ok) {
                throw new Error(json.error || `pairing ${res.status}`);
              }
              return json;
            }
            return undefined;
          },
        },
      });
    });
    await page.goto("/harness/graph");
    await expect(page.getByRole("button", { name: "Pair this desktop" })).toBeVisible();
    await page.getByRole("button", { name: "Pair this desktop" }).click();
    await expect(page.getByText(/Paired device /)).toBeVisible();
    const calls = await page.evaluate(
      () =>
        (window as unknown as { __TAURI_PAIRING_CALLS__: Array<{ cmd: string; args: Record<string, string> }> })
          .__TAURI_PAIRING_CALLS__,
    );
    expect(calls.some((c) => c.cmd === "harness_pairing_origin_configured")).toBeTruthy();
    const remotePosts = calls.filter((c) => c.cmd === "harness_pairing_post");
    expect(remotePosts.map((c) => c.args.path)).toEqual([
      "/api/harness/pairing/sso/begin",
      "/api/harness/pairing/sso/poll",
      "/api/harness/pairing/challenges",
      "/api/harness/pairing/confirm",
    ]);
    const keychain = calls.filter((c) => c.cmd === "harness_device_keychain_set");
    expect(keychain).toHaveLength(1);
    expect(keychain[0]?.args.account).toMatch(
      /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/,
    );
    expect(keychain[0]?.args.token).toMatch(/^[A-Za-z0-9_-]{43}$/);
    const storedId = await page.evaluate(() => sessionStorage.getItem("anycode_harness_device_id"));
    expect(storedId).toBe(keychain[0]?.args.account);
    const blob = await page.evaluate(() => JSON.stringify(sessionStorage));
    expect(blob).not.toContain(keychain[0]?.args.token);

    const account = keychain[0]!.args.account;
    const token = keychain[0]!.args.token;
    const written = helperOp({
      op: "keychain_set",
      service: "anycode.harness.device",
      account,
      secret: token,
    });
    expect(written.ok).toBe(true);
    const read = helperOp({
      op: "keychain_get",
      service: "anycode.harness.device",
      account,
    });
    expect(read.ok).toBe(true);
    expect(read.text).toBe(token);
    spawnSync("security", [
      "delete-generic-password",
      "-s",
      "anycode.harness.device",
      "-a",
      account,
    ]);
  });
});
