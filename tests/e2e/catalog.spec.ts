import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

const FAMILIES = ["DLSS", "FSR", "XeSS", "Reflex"];

test.describe("catalog", () => {
  test("vendor families and version pickers render", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "catalog");

    await expect(page.locator(".vendor-card").first()).toBeVisible();
    await expect(page.locator(".feature-row, .feature-row-btn").first()).toBeVisible();

    let visibleFamilies = 0;
    for (const fam of FAMILIES) {
      if (await page.getByText(fam, { exact: false }).first().count()) visibleFamilies++;
    }
    expect(visibleFamilies).toBeGreaterThanOrEqual(2);

    const picker = page.locator(".feature-row-btn").first();
    if (await picker.count()) {
      await picker.click();
      await expect(
        page.locator(".glass-dialog, [class*=flyout], [class*=popover], [role=menu]").first(),
      ).toBeVisible();
      await page.keyboard.press("Escape");
    }
  });

  test("DirectStorage advanced catalog entry renders from the embedded manifest", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "catalog");
    await page.locator(".runtime-search input").fill("DirectStorage");
    const microsoftCard = page.locator(".vendor-card", { hasText: "Microsoft" }).first();
    await expect(microsoftCard).toBeVisible();
    await microsoftCard.locator(".feature-row-btn.is-advanced").click();
    const directStorage = page.getByRole("button", { name: "View versions of DirectStorage", exact: true });
    await expect(directStorage).toBeVisible();
    await directStorage.click();
    await expect(page.getByRole("button", { name: /Download/i }).first()).toBeVisible();
  });
});
