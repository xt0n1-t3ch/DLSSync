import { test, expect, type Page } from "./fixtures";
import { gotoView } from "./helpers";

const MASTER_LABEL = "Enable background scanning";
const INTERVAL_LABEL = "Scan every";
const NOTIFY_LABEL = "Windows notification";
const CLOSE_TO_TRAY_LABEL = "Close to tray";
const RUN_AT_STARTUP_LABEL = "Start with Windows";

function rowControl(page: Page, labelText: string) {
  return page
    .locator(".row", { has: page.locator(".row-label", { hasText: labelText }) })
    .locator('input[type="checkbox"], select')
    .first();
}

function backgroundAutoApplyControl(page: Page) {
  return page
    .locator(".row", { hasText: "Auto-apply updates" })
    .filter({ hasText: "background scan" })
    .locator('input[type="checkbox"]')
    .first();
}

test.describe("settings daemon", () => {
  test("background-update gating disables dependent controls when master is off", async ({
    app,
  }) => {
    const { page } = app;
    await gotoView(page, "settings");
    await page.locator(".side-tab", { hasText: /general/i }).first().click();

    await expect(page.getByText("Background updates").first()).toBeVisible();

    const master = rowControl(page, MASTER_LABEL);
    await expect(master).toBeVisible();
    if (await master.isChecked()) {
      await master.uncheck();
    }

    await expect(rowControl(page, INTERVAL_LABEL)).toBeDisabled();
    await expect(rowControl(page, NOTIFY_LABEL)).toBeDisabled();
    await expect(backgroundAutoApplyControl(page)).toBeDisabled();

    await expect(rowControl(page, CLOSE_TO_TRAY_LABEL)).toBeEnabled();
    await expect(rowControl(page, RUN_AT_STARTUP_LABEL)).toBeEnabled();
  });
});
