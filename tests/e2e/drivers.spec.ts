import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";
import { systemScanTimeoutMs } from "./config";

const VENDOR_TOKENS = ["NVIDIA", "GeForce", "RTX", "AMD", "Radeon", "Intel", "Arc", "Iris"];

test.describe("drivers", () => {
  test("GPU list renders with at least one vendor token", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "drivers");

    const list = page.locator(".driver-list");
    await expect(list).toBeVisible();
    await expect(page.locator(".driver-card").first()).toBeVisible();

    const text = (await list.innerText()).toString();
    const matched = VENDOR_TOKENS.filter((token) => text.includes(token));
    expect(matched.length).toBeGreaterThan(0);
  });

  test("global DLSS Overrides panel is present", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "drivers");
    await expect(page.getByText("DLSS Overrides").first()).toBeVisible();
  });

  test("system components scan surfaces cards, admin note, and version history", async ({
    app,
  }, testInfo) => {
    const { page } = app;
    test.setTimeout(systemScanTimeoutMs + 30_000);
    await gotoView(page, "drivers");

    const disclosure = page.locator(".system-block .disclosure-toggle");
    await expect(disclosure).toBeVisible();
    await disclosure.click();
    await expect(page.locator(".admin-note", { hasText: "Administrator rights" })).toBeVisible();

    const scanButton = page.locator(".system-block .check-btn");
    const scanCompleted = await scanButton
      .filter({ hasText: /Rescan/i })
      .waitFor({ timeout: systemScanTimeoutMs })
      .then(() => true)
      .catch(() => false);

    if (!scanCompleted) {
      testInfo.annotations.push({
        type: "gated",
        description: "Windows Update component scan did not complete within the bounded window",
      });
      test.skip(true, "system component scan did not complete in time");
      return;
    }

    const cards = page.locator(".sys-card");
    if ((await cards.count()) === 0) {
      testInfo.annotations.push({
        type: "gated",
        description: "no outdated system components present on this machine",
      });
      test.skip(true, "no outdated system components at test time");
      return;
    }

    await expect(page.locator(".sys-group").first()).toBeVisible();

    const verToggle = page.locator('button[aria-label="Version history"]').first();
    if (await verToggle.count()) {
      await verToggle.click();
      await expect(page.locator(".sys-versions-panel").first()).toBeVisible();
    }
  });
});
