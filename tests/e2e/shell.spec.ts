import { test, expect } from "./fixtures";
import { appVersion } from "./config";
import type { ViewName } from "./helpers";

const NAV_VIEWS: ViewName[] = ["library", "catalog", "drivers", "backups", "settings", "about"];

test.describe("shell", () => {
  test("app shell mounts with all six nav items", async ({ app }) => {
    await expect(app.page.locator(".app-shell")).toBeVisible();
    for (const view of NAV_VIEWS) {
      await expect(app.page.getByTestId(`nav-${view}`)).toBeVisible();
    }
  });

  test("sidebar brand shows the current app version", async ({ app }) => {
    await expect(app.page.locator(".sidebar-brand .brand-version")).toHaveText(`v${appVersion}`);
  });
});
