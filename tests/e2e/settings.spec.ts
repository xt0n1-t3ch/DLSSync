import { test, expect } from "./fixtures";
import { appVersion } from "./config";
import { gotoView } from "./helpers";

test.describe("settings", () => {
  test("hero version, section headings, toggles, and tab switching work", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "settings");

    await expect(page.locator(".settings-hero .hero-version")).toHaveText(`v${appVersion}`);

    expect(await page.locator(".section-title-h").count()).toBeGreaterThanOrEqual(2);
    expect(await page.locator(".side-tab input[type=checkbox], .card input[type=checkbox], .seg-btn").count()).toBeGreaterThan(0);

    const updatesTab = page.locator(".side-tab", { hasText: /update/i }).first();
    await updatesTab.click();
    await expect(updatesTab).toHaveClass(/active/);
    await expect(page.locator(".section-title-h").first()).toBeVisible();
  });
});
