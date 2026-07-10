import { E2E_PROTECTED_GAME_NAME, test, expect } from "./fixtures";
import { gotoView, openGameCard } from "./helpers";

test.describe("game detail", () => {
  test("opens full detail with feature rows and returns via back", async ({ app }, testInfo) => {
    const { page } = app;
    await gotoView(page, "library");

    if ((await page.locator(".game-card").count()) === 0) {
      testInfo.annotations.push({ type: "gated", description: "no games in library" });
      test.skip(true, "no games in library at test time");
      return;
    }

    await openGameCard(page, 0);
    const detail = page.locator(".detail-view");
    await expect(page.locator(".detail-back")).toBeVisible();
    await expect(page.locator(".drawer-body")).toBeVisible();
    await expect(page.locator(".drawer-foot")).toHaveCount(1);

    const featureRows = await page.locator(".feature-row").count();
    if (featureRows > 0) {
      await expect(page.locator(".feature-row").first()).toBeVisible();
      await expect(page.locator(".summary-row")).toHaveCount(1);
    } else {
      testInfo.annotations.push({
        type: "gated",
        description: "first game has no scanned upscaler DLLs; feature rows / summary row absent",
      });
    }

    await page.locator(".detail-back").click();
    await expect(detail).toHaveCount(0);
    await expect(page.locator(".game-card").first()).toBeVisible();
  });

  test("anti-cheat apply-risk affordance present on a guarded game", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");

    const protectedCard = page.locator(".game-card", { hasText: E2E_PROTECTED_GAME_NAME }).first();
    await expect(protectedCard).toBeVisible();
    await expect(protectedCard).toContainText("DLSS", { timeout: 30_000 });
    const detail = page.locator(".detail-view");
    await protectedCard.locator(".body").click();
    await expect(detail).toBeVisible();
    await expect(page.locator(".drawer-body .warning-banner, .detail-view .warning-banner").first()).toBeVisible();
    await expect(page.locator(".ac-apply-risk, .ac-apply-confirm").first()).toBeVisible();
  });

  test("anti-cheat confirmation opens the signed update plan before mutation", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");

    const protectedCard = page.locator(".game-card", { hasText: E2E_PROTECTED_GAME_NAME }).first();
    await expect(protectedCard).toContainText("DLSS", { timeout: 30_000 });
    await protectedCard.locator(".body").click();

    const apply = page.locator(".foot-apply");
    await expect(apply).toBeEnabled();
    await apply.click();
    await page.locator(".ac-confirm-proceed").click();

    await expect(page.getByRole("heading", { name: "Review update plan", exact: true })).toBeVisible();
    await expect(page.locator(".plan-proof")).toContainText("Ed25519");
    await expect(page.getByRole("button", { name: "Apply selected", exact: true })).toBeEnabled();

    // Close the modal so it never leaks into the shared worker-scoped app and
    // blocks the next spec's navigation.
    await page.locator(".plan-modal .close").click();
    await expect(page.getByRole("heading", { name: "Review update plan", exact: true })).toBeHidden();
  });
});
