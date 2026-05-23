import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.DASHBOARD_E2E_PORT ?? "43199");
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        channel: process.env.CI ? undefined : "chrome",
      },
    },
  ],
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  webServer: {
    command: `bash ../../scripts/dashboard-e2e-server.sh ${port}`,
    wait: { stdout: /e2e-fixture-ready/ },
    stdout: "pipe",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
