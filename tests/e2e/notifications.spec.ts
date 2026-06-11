import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("notifications", () => {
  test("bell opens the notifications panel", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");

    await page.locator("[data-notifications-toggle]").click();
    await expect(page.locator(".bell-panel")).toBeVisible();

    await page.keyboard.press("Escape");
  });
});
