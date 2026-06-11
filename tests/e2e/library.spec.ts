import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("library", () => {
  test("renders cards, segmented view toggle, and supports grid/list switch", async ({ app }, testInfo) => {
    const { page } = app;
    await gotoView(page, "library");

    await page
      .locator(".game-card")
      .first()
      .waitFor({ state: "visible", timeout: 15_000 })
      .catch(() => undefined);
    if ((await page.locator(".game-card").count()) === 0) {
      testInfo.annotations.push({ type: "gated", description: "no games in library" });
      test.skip(true, "no games in library at test time");
      return;
    }
    await expect(page.locator(".game-card").first()).toBeVisible();
    const segButtons = page.locator(".seg-btn");
    expect(await segButtons.count()).toBeGreaterThanOrEqual(2);

    const listToggle = page.getByRole("button", { name: /^list$/i }).first();
    await listToggle.click();
    await expect(page.locator(".list").first()).toBeVisible();

    const gridToggle = page.getByRole("button", { name: /^grid$/i }).first();
    await gridToggle.click();
    await expect(page.locator(".grid").first()).toBeVisible();
  });

  test("search input and filter controls are present", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");
    await expect(page.locator("header input[type=search]")).toBeVisible();
    await expect(page.locator(".filter-toolbar")).toBeVisible();
  });

  test("apply-all affordance appears only when pending updates exist", async ({ app }, testInfo) => {
    const { page } = app;
    await gotoView(page, "library");
    const heroCount = await page.locator(".updates-hero").count();
    if (heroCount === 0) {
      testInfo.annotations.push({
        type: "gated",
        description: "no games in library at test time; status hero absent",
      });
      test.skip(true, "no games at test time");
      return;
    }
    await expect(page.locator(".updates-hero .display-num")).toBeVisible();
    const pendingTone = await page
      .locator('.updates-hero .display-num[data-tone="warning"]')
      .count();
    if (pendingTone === 0) {
      await expect(page.locator(".updates-hero-apply")).toHaveCount(0);
      testInfo.annotations.push({
        type: "gated",
        description: "no outdated DLLs at test time; hero renders all-clear without apply CTA",
      });
      return;
    }
    await expect(page.locator(".updates-hero-apply, .btn-apply-all").first()).toBeVisible();
  });
});
