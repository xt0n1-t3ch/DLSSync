import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("operation journal", () => {
  test("renders filters and the persisted startup history", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "journal");

    await expect(page.getByRole("heading", { name: "Operation Journal", exact: true })).toBeVisible();
    await expect(page.getByLabel("Operation")).toBeVisible();
    await expect(page.getByLabel("Result")).toBeVisible();
    await expect(page.getByRole("button", { name: "Copy redacted JSON", exact: true })).toBeVisible();
    await expect(page.locator(".journal-entry, .journal-empty").first()).toBeVisible();
  });
});
