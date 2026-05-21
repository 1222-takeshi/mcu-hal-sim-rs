const { test, expect } = require("@playwright/test");

async function postWiring(page, body) {
  let lastError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const status = await page.evaluate(async (payload) => {
        const response = await fetch("/api/wiring", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        return response.status;
      }, body);
      expect(status).toBe(200);
      lastError = undefined;
      break;
    } catch (error) {
      lastError = error;
      await page.waitForTimeout(100);
    }
  }
  if (lastError) throw lastError;
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

async function waitForDashboardReady(page) {
  await page.waitForFunction(() => {
    const stext = document.querySelector("#stext");
    return stext && stext.textContent.includes("Online");
  }, { timeout: 5000 });
}

test.describe("device dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await gotoDashboard(page);
    await postWiring(page, { sensor_profile: "full" });
    await page.reload({ waitUntil: "load" });
    await waitForDashboardReady(page);
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
    await expect(page.locator("#wiring-svg-wrap svg .dev-pin")).toHaveCount(0);
  });

  test("applies profile presets to the page and wiring endpoint", async ({ page }) => {
    await page.locator("#sensor-profile-select").selectOption("minimal");

    await expect(page.locator("#light-card")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#servo-hw-item")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#device-toggle-list input[data-device-kind]:checked")).toHaveCount(2);
    await expect(page.locator("#wiring-svg-wrap svg")).toHaveAttribute("viewBox", "0 0 580 520");
    await expect(page.locator("#wiring-svg-wrap svg .dev-pin")).toHaveCount(0);
    await expect(page.locator("#wiring-svg-wrap svg .w-vcc.w-bus-branch")).toHaveCount(2);
    await expect(page.locator("#wiring-svg-wrap svg .w-gnd.w-bus-branch")).toHaveCount(2);
    await expect(page.locator("#wiring-svg-wrap svg .w-sda.w-bus-branch")).toHaveCount(2);
    await expect(page.locator("#wiring-svg-wrap svg .w-scl.w-bus-branch")).toHaveCount(2);

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
    await waitForDashboardReady(page);

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

  test("keeps DS3231 wiring and disabled actuator state consistent across APIs", async ({
    page,
  }) => {
    await postWiring(page, { selected_devices: ["ds3231"] });
    await page.reload({ waitUntil: "load" });
    await waitForDashboardReady(page);

    await expect
      .poll(async () => {
        const [wiring, svg, state] = await page.evaluate(async () => {
          const [wiringResponse, svgResponse, stateResponse] = await Promise.all([
            fetch("/api/wiring"),
            fetch("/api/wiring/svg"),
            fetch("/api/state"),
          ]);
          return Promise.all([
            wiringResponse.json(),
            svgResponse.text(),
            stateResponse.json(),
          ]);
        });

        return JSON.stringify({
          wiringAddress: wiring.devices.find((device) => device.kind === "ds3231")?.address,
          svgHasDs3231LogicalAddress: svg.includes("DS3231") && svg.includes("0x68"),
          recentOperationsUseLogicalAddress:
            state.i2c.recent_operations.length > 0 &&
            state.i2c.recent_operations.every((line) => line.includes("0x68")),
          servoAngle: state.servo.angle_degrees,
          leftMotor: state.motor_driver.left,
          rightMotor: state.motor_driver.right,
        });
      })
      .toBe(
        JSON.stringify({
          wiringAddress: "0x68",
          svgHasDs3231LogicalAddress: true,
          recentOperationsUseLogicalAddress: true,
          servoAngle: 0,
          leftMotor: { direction: "coast", duty_percent: 0 },
          rightMotor: { direction: "coast", duty_percent: 0 },
        }),
      );
  });

  test("resets actuator state after deselection", async ({ page }) => {
    await expect(page.locator("#servo-value")).not.toHaveText("-- deg");
    await postWiring(page, { selected_devices: ["ds3231"] });
    await page.reload({ waitUntil: "load" });
    await waitForDashboardReady(page);

    await expect(page.locator("#servo-hw-item")).toHaveJSProperty("hidden", true);
    await expect(page.locator("#motor-left-item")).toHaveJSProperty("hidden", true);

    await expect
      .poll(async () => {
        const state = await page.evaluate(async () => {
          const response = await fetch("/api/state");
          return response.json();
        });
        return JSON.stringify({
          servo: state.servo.angle_degrees,
          left: state.motor_driver.left,
          right: state.motor_driver.right,
        });
      })
      .toBe(
        JSON.stringify({
          servo: 0,
          left: { direction: "coast", duty_percent: 0 },
          right: { direction: "coast", duty_percent: 0 },
        }),
      );
  });

  test("surfaces wiring update failures in the status bar", async ({ page }) => {
    await page.route("**/api/wiring", async (route) => {
      const request = route.request();
      if (request.method() !== "POST") {
        await route.continue();
        return;
      }
      const body = request.postDataJSON();
      if (Array.isArray(body.selected_devices) && body.selected_devices.includes("bh1750")) {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });

    await postWiring(page, { sensor_profile: "minimal" });
    await page.reload({ waitUntil: "load" });
    await waitForDashboardReady(page);
    await page.locator('#device-toggle-list input[data-device-kind="bh1750"]').check();
    await expect(page.locator("#serr")).toContainText("Device toggle update failed");
  });
});

test("serializes rapid profile and device toggle updates so the latest choice wins", async ({ page }) => {
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
  await waitForDashboardReady(page);
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
    const first = bh1750Toggle.dispatchEvent(new Event("change", { bubbles: true }));
    profileSel.value = "full";
    const second = profileSel.dispatchEvent(new Event("change", { bubbles: true }));
    return { first, second };
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
