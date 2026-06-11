import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("command palette", () => {
  test("opens, accepts a query, and lists results", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");

    await page.locator(".palette-btn").click();
    const palette = page.locator(".palette");
    await expect(palette).toBeVisible();

    await palette.locator("input").fill("back");
    await expect(palette.locator(".result").first()).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(palette).toHaveCount(0);
  });
});
