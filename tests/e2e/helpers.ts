import { expect, type Page } from "./fixtures";

export type ViewName =
  | "library"
  | "catalog"
  | "drivers"
  | "backups"
  | "settings"
  | "about";

export async function gotoView(page: Page, view: ViewName): Promise<void> {
  await page.getByTestId(`nav-${view}`).click();
  await expect(page.getByTestId(`view-${view}`)).toBeVisible();
}

export async function openGameCard(page: Page, index: number): Promise<void> {
  const card = page.locator(".game-card").nth(index);
  await expect(card).toBeVisible();
  await card.locator(".body").click();
  await expect(page.locator(".detail-view")).toBeVisible();
}
