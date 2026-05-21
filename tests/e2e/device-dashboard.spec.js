const { test, expect } = require("@playwright/test");

async function postWiring(page, body) {
  const status = await page.evaluate(async (payload) => {
    const response = await fetch("/api/wiring", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    return response.status;
  }, body);
  expect(status).toBe(200);
}

async function getWiring(page) {
  return page.evaluate(async () => {
    const response = await fetch("/api/wiring");
    return response.json();
  });
}

async function gotoDashboard(page) {
  let lastError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await page.goto("/", { waitUntil: "load" });
      lastError = undefined;
      break;
    } catch (error) {
      lastError = error;
      await page.waitForTimeout(250);
    }
  }
  if (lastError) throw lastError;
  await expect(page.locator("#device-toggle-list input[data-device-kind]")).toHaveCount(11);
  await expect(page.locator("#wiring-svg-wrap svg")).toBeVisible();
  await expect(page.locator("#stext")).toContainText("Online");
}

test.describe("device dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await gotoDashboard(page);
    await postWiring(page, { sensor_profile: "full" });
    await page.reload({ waitUntil: "load" });
    await expect(page.locator("#device-toggle-list input[data-device-kind]:checked")).toHaveCount(11);
    await expect(page.locator("#light-card")).toHaveJSProperty("hidden", false);
  });

  test("renders the live dashboard with shared bus wiring", async ({ page }) => {
    await expect(page.locator("#light-card")).toHaveJSProperty("hidden", false);
    await expect(page.locator("#servo-hw-item")).toHaveJSProperty("hidden", false);
    await expect(page.locator("#wiring-svg-wrap svg .w-vcc.w-bus-trunk")).toHaveCount(1);
    await expect(page.locator("#wiring-svg-wrap svg .w-gnd.w-bus-trunk")).toHaveCount(1);
    await expect(page.locator("#wiring-svg-wrap svg .w-sda.w-bus-trunk")).toHaveCount(1);
    await expect(page.locator("#wiring-svg-wrap svg .w-scl.w-bus-trunk")).toHaveCount(1);
  });

  test("applies profile presets to the page and wiring endpoint", async ({ page }) => {
    await page.locator("#sensor-profile-select").selectOption("minimal");

    await expect(page.locator("#light-card")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#servo-hw-item")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#device-toggle-list input[data-device-kind]:checked")).toHaveCount(2);
    await expect(page.locator("#wiring-svg-wrap svg")).toHaveAttribute("viewBox", "0 0 580 520");

    await expect
      .poll(async () => {
        const data = await page.evaluate(async () => {
          const response = await fetch("/api/wiring");
          return response.json();
        });
        return JSON.stringify({
          sensor_profile: data.sensor_profile,
          selected_devices: data.selected_devices,
        });
      })
      .toBe(
        JSON.stringify({
          sensor_profile: "minimal",
          selected_devices: ["bme280", "lcd1602"],
        }),
      );
  });

  test("persists device toggle overrides and refreshes the diagram", async ({ page }) => {
    await page.locator("#sensor-profile-select").selectOption("minimal");
    await expect
      .poll(async () => {
        const data = await page.evaluate(async () => {
          const response = await fetch("/api/wiring");
          return response.json();
        });
        return data.selected_devices.join(",");
      })
      .toBe("bme280,lcd1602");

    await page.locator('#device-toggle-list input[data-device-kind="bh1750"]').check();

    await expect(page.locator("#light-card")).toHaveJSProperty("hidden", false);
    await expect(
      page.locator("#wiring-svg-wrap svg text").filter({ hasText: "BH1750" }),
    ).toHaveCount(1);

    await expect
      .poll(async () => {
        const data = await page.evaluate(async () => {
          const response = await fetch("/api/wiring");
          return response.json();
        });
        return data.selected_devices.join(",");
      })
      .toBe("bme280,lcd1602,bh1750");
  });

  test("keeps BME280 values live when the LCD is disabled", async ({ page }) => {
    await postWiring(page, { selected_devices: ["bme280"] });
    await page.reload({ waitUntil: "load" });

    await expect(page.locator("#device-toggle-list input[data-device-kind]:checked")).toHaveCount(1);
    await expect(page.locator("#device-toggle-list input[data-device-kind=\"bme280\"]")).toBeChecked();
    await expect(page.locator("#servo-hw-item")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#temp-value")).not.toHaveText("--");
    await expect(page.locator("#lcd-line-1")).toHaveText("                ");

    await expect
      .poll(async () => {
        const data = await page.evaluate(async () => {
          const response = await fetch("/api/state");
          return response.json();
        });
        return JSON.stringify({
          selected_devices: data.wiring.selected_devices,
          temperature_c: data.climate.temperature_c != null,
          lcd_line_1: data.climate.physical_lcd_frame[0],
        });
      })
      .toBe(
        JSON.stringify({
          selected_devices: ["bme280"],
          temperature_c: true,
          lcd_line_1: "                ",
        }),
      );
  });
});

test("serializes rapid profile and device toggle updates so the latest choice wins", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const nativeFetch = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      if (
        typeof input === "string" &&
        input === "/api/wiring" &&
        init?.method === "POST" &&
        typeof init.body === "string"
      ) {
        const payload = JSON.parse(init.body);
        if (
          Array.isArray(payload.selected_devices) &&
          payload.selected_devices.length === 3 &&
          payload.selected_devices.includes("bh1750")
        ) {
          await new Promise((resolve) => setTimeout(resolve, 200));
        }
      }

      return nativeFetch(input, init);
    };
  });

  await gotoDashboard(page);
  await postWiring(page, { sensor_profile: "minimal" });
  await page.reload({ waitUntil: "load" });
  await expect(page.locator("#sensor-profile-select")).toHaveValue("minimal");
  await expect(
    page.locator('#device-toggle-list input[data-device-kind="bh1750"]'),
  ).toBeVisible();

  await page.evaluate(() => {
    const bh1750Toggle = document.querySelector(
      '#device-toggle-list input[data-device-kind="bh1750"]',
    );
    const profileSel = document.getElementById("sensor-profile-select");
    if (!bh1750Toggle || !profileSel) {
      throw new Error("dashboard controls missing");
    }

    bh1750Toggle.checked = true;
    const first = changeDeviceToggle();
    profileSel.value = "full";
    const second = changeWiringConfig();
    return Promise.all([first, second]);
  });

  await expect
    .poll(async () => {
      const data = await getWiring(page);
      return JSON.stringify({
        sensor_profile: data.sensor_profile,
        selected_devices: data.selected_devices.length,
      });
    })
    .toBe(
      JSON.stringify({
        sensor_profile: "full",
        selected_devices: 11,
      }),
    );

  await expect(page.locator("#light-card")).toHaveJSProperty("hidden", false);
});
