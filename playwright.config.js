const { defineConfig, devices } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never" }], ["list"]],
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL || "http://127.0.0.1:4173",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
      testMatch: "**/cross-browser-smoke.spec.js",
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
      testMatch: "**/cross-browser-smoke.spec.js",
    },
  ],
  webServer: process.env.PLAYWRIGHT_BASE_URL
    ? undefined
    : {
        command: "cargo run -p platform-pc-sim --bin device-dashboard-web -- 4173",
        url: "http://127.0.0.1:4173",
        reuseExistingServer: false,
        timeout: 120000,
      },
});
