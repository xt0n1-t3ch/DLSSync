import { test, expect } from "./fixtures";
import { appVersion } from "./config";
import { gotoView } from "./helpers";

test.describe("about", () => {
  test("shows app version, manifest sources, and system info", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "about");

    await expect(page.getByText(`v${appVersion}`, { exact: false }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "GitHub", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Manifest", exact: true })).toBeVisible();
    await expect(page.locator(".source-card").first()).toBeVisible();
    await expect(page.getByText("Your system", { exact: false }).first()).toBeVisible();
  });
});
