import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("backups", () => {
  test("hero, search, and group-by toolbar render with content or empty state", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "backups");

    const hasGroups = (await page.locator(".group-row").count()) > 0;
    const hasEmpty = (await page.locator(".empty").count()) > 0;
    expect(hasGroups || hasEmpty).toBe(true);

    if (hasGroups) {
      await expect(page.locator(".backup-search input").first()).toBeVisible();
      await expect(page.locator(".backup-hero")).toBeVisible();
      await expect(page.locator(".group-by-toggle").first()).toBeVisible();
    }
  });
});
