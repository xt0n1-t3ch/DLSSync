import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { E2E_PROTECTED_GAME_NAME, test, expect } from "./fixtures";
import { repoRoot } from "./config";
import { gotoView } from "./helpers";

const captureEnabled = process.env.DLSSYNC_CAPTURE_MARKETING === "1";
const gallery = join(repoRoot, ".github", "assets", "nexus", "gallery");

test.describe("marketing screenshots", () => {
  test.skip(!captureEnabled, "set DLSSYNC_CAPTURE_MARKETING=1 to replace the Nexus gallery");

  test("captures the final product surfaces from the real WebView2 app", async ({ app }) => {
    const { page } = app;
    mkdirSync(gallery, { recursive: true });
    await page.setViewportSize({ width: 1554, height: 990 });

    await gotoView(page, "library");
    const protectedCard = page.locator(".game-card", { hasText: E2E_PROTECTED_GAME_NAME }).first();
    await expect(protectedCard).toContainText("DLSS", { timeout: 30_000 });
    await expect(page.locator(".game-card")).toHaveCount(4, { timeout: 30_000 });
    await page.screenshot({ path: join(gallery, "01-library.png"), animations: "disabled" });

    await protectedCard.locator(".body").click();
    await expect(page.locator(".detail-view")).toBeVisible();
    await expect(page.locator(".detail-view .feature-row")).toHaveCount(2, { timeout: 30_000 });
    await page.screenshot({ path: join(gallery, "02-game-detail.png"), animations: "disabled" });

    const apply = page.locator(".foot-apply");
    await apply.click();
    await page.locator(".ac-confirm-proceed").click();
    await expect(page.getByRole("heading", { name: "Review update plan", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Apply selected", exact: true })).toBeEnabled();
    await page.screenshot({ path: join(gallery, "03-update-plan.png"), animations: "disabled" });
    await page.locator(".plan-modal .close").click();

    await gotoView(page, "catalog");
    await expect(page.getByRole("heading", { name: "Trust Center", exact: true })).toBeVisible();
    await page.screenshot({ path: join(gallery, "04-catalog-trust.png"), animations: "disabled" });

    await gotoView(page, "drivers");
    await expect(page.locator(".drivers-page, [data-testid='view-drivers']")).toBeVisible();
    await page.screenshot({ path: join(gallery, "05-drivers.png"), animations: "disabled" });

    await gotoView(page, "journal");
    await expect(page.getByRole("heading", { name: "Operation Journal", exact: true })).toBeVisible();
    await page.screenshot({ path: join(gallery, "06-journal.png"), animations: "disabled" });

    await gotoView(page, "settings");
    await page.screenshot({ path: join(gallery, "07-settings.png"), animations: "disabled" });

    await gotoView(page, "about");
    await expect(page.getByRole("button", { name: "GitHub", exact: true })).toBeVisible();
    await page.screenshot({ path: join(gallery, "08-about.png"), animations: "disabled" });
  });
});
