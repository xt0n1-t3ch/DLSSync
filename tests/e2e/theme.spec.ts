import { test, expect } from "./fixtures";
import { gotoView } from "./helpers";

test.describe("theme", () => {
  test("toggle flips the document theme and back", async ({ app }) => {
    const { page } = app;
    await gotoView(page, "library");

    const root = page.locator("html");
    const before = await root.getAttribute("data-theme");

    const toggle = page.getByRole("button", { name: "Toggle theme" }).first();
    await toggle.click();
    await expect(root).not.toHaveAttribute("data-theme", before ?? "");

    await toggle.click();
    await expect(root).toHaveAttribute("data-theme", before ?? "");
  });
});
