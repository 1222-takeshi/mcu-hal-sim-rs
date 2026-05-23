/**
 * Cross-browser smoke tests for device-dashboard-web.
 *
 * These tests run on Chromium, Firefox, and WebKit (see playwright.config.js).
 * They are intentionally lightweight — the goal is to catch browser-specific
 * regressions in the SSE connection, SVG rendering, and basic UI interactions.
 * Full regression coverage lives in device-dashboard.spec.js (Chromium only).
 */
const { test, expect } = require("@playwright/test");

const CLEAN_STATE = {
  board: "original-esp32",
  sensor_profile: "full",
  show_bus_labels: false,
};

async function resetServerState(request) {
  await request.post("/api/wiring", {
    data: CLEAN_STATE,
    headers: { "Content-Type": "application/json" },
  });
}

async function waitForOnline(page) {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await page.goto("/", { waitUntil: "load" });
      break;
    } catch (_e) {
      await page.waitForTimeout(250);
    }
  }
  await expect(page.locator("#stext")).toContainText("Online", { timeout: 20000 });
  await expect(page.locator("#wiring-svg-wrap svg")).toBeVisible();
}

test.describe("cross-browser smoke", () => {
  test.beforeEach(async ({ request }) => {
    await resetServerState(request);
  });

  test.afterAll(async ({ request }) => {
    // Restore clean server state after all smoke tests so subsequent test files start fresh.
    await resetServerState(request);
  });

  test("dashboard loads and shows SSE-connected status", async ({ page }) => {
    await waitForOnline(page);
  });

  test("device toggle list is populated", async ({ page }) => {
    await waitForOnline(page);
    await expect(
      page.locator("#device-toggle-list input[data-device-kind]"),
    ).toHaveCount(11);
  });

  test("wiring SVG renders shared bus trunks", async ({ page }) => {
    await waitForOnline(page);
    // Full profile (set by beforeEach) guarantees 11 I2C devices and all four bus trunks.
    await expect(
      page.locator("#device-toggle-list input[data-device-kind]:checked"),
    ).toHaveCount(11, { timeout: 10000 });
    await expect(
      page.locator("#wiring-svg-wrap svg .w-sda.w-bus-trunk"),
    ).toHaveCount(1);
    await expect(
      page.locator("#wiring-svg-wrap svg .w-scl.w-bus-trunk"),
    ).toHaveCount(1);
    await expect(
      page.locator("#wiring-svg-wrap svg .w-vcc.w-bus-trunk"),
    ).toHaveCount(1);
    await expect(
      page.locator("#wiring-svg-wrap svg .w-gnd.w-bus-trunk"),
    ).toHaveCount(1);
  });

  test("profile selector switches device count", async ({ page }) => {
    await waitForOnline(page);
    // beforeEach has already set full profile; confirm the UI reflects it.
    await expect(
      page.locator("#device-toggle-list input[data-device-kind]:checked"),
    ).toHaveCount(11, { timeout: 10000 });

    // Use waitForResponse to ensure the POST /api/wiring completes before asserting.
    // This is required for WebKit where the async fetch may not be immediately visible.
    await Promise.all([
      page.waitForResponse(
        (resp) =>
          resp.url().includes("/api/wiring") &&
          resp.request().method() === "POST",
        { timeout: 10000 },
      ),
      page.locator("#sensor-profile-select").selectOption("minimal"),
    ]);
    await expect(
      page.locator("#device-toggle-list input[data-device-kind]:checked"),
    ).toHaveCount(2, { timeout: 10000 });
  });

  test("SSE stream delivers climate sensor readings", async ({ page }) => {
    await waitForOnline(page);
    // beforeEach ensures bme280 is active via full profile.
    await expect(
      page.locator("#device-toggle-list input[data-device-kind]:checked"),
    ).toHaveCount(11, { timeout: 10000 });
    await expect(page.locator("#temp-value")).not.toContainText("--", {
      timeout: 10000,
    });
    await expect(page.locator("#hum-value")).not.toContainText("--", {
      timeout: 10000,
    });
  });

  test("wiring diagram has no duplicate per-device pin labels", async ({ page }) => {
    await waitForOnline(page);
    // show_bus_labels=false (set by beforeEach) means no per-device .dev-pin labels.
    await expect(
      page.locator("#wiring-svg-wrap svg .dev-pin"),
    ).toHaveCount(0);
  });
});
