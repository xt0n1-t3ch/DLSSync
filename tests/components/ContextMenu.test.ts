import { describe, it, expect, beforeEach, vi } from "vitest";
import { tick } from "svelte";
import { render, fireEvent } from "@testing-library/svelte";
import ContextMenu, { type ContextMenuItem } from "@/components/ContextMenu.svelte";

const items: ContextMenuItem[] = [
  { action: "open_folder", label: "Open folder" },
  { action: "scan", label: "Scan" },
  { action: "pin", label: "Pin a version…" },
  { action: "hide", label: "Hide" },
];

beforeEach(() => {
  Object.defineProperty(window, "innerWidth", { value: 1280, configurable: true });
  Object.defineProperty(window, "innerHeight", { value: 720, configurable: true });
});

describe("ContextMenu", () => {
  it("renders a menu with one menuitem per action", async () => {
    const { getByRole, getAllByRole } = render(ContextMenu, {
      props: { x: 40, y: 40, items, onSelect: vi.fn(), onClose: vi.fn() },
    });
    await tick();
    expect(getByRole("menu")).toBeTruthy();
    const menuItems = getAllByRole("menuitem");
    expect(menuItems.length).toBe(4);
    expect(menuItems.map((b) => b.textContent?.trim())).toEqual([
      "Open folder",
      "Scan",
      "Pin a version…",
      "Hide",
    ]);
  });

  it("fires onSelect with the chosen action and then closes", async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    const { getAllByRole } = render(ContextMenu, {
      props: { x: 40, y: 40, items, onSelect, onClose },
    });
    await tick();
    await fireEvent.click(getAllByRole("menuitem")[1]);
    expect(onSelect).toHaveBeenCalledWith("scan");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("moves focus down/up with the arrow keys (wrapping)", async () => {
    const { getByRole, getAllByRole } = render(ContextMenu, {
      props: { x: 40, y: 40, items, onSelect: vi.fn(), onClose: vi.fn() },
    });
    await tick();
    const menu = getByRole("menu");
    const menuItems = getAllByRole("menuitem");
    expect(document.activeElement).toBe(menuItems[0]);
    await fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toBe(menuItems[1]);
    await fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(document.activeElement).toBe(menuItems[0]);
    await fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(document.activeElement).toBe(menuItems[menuItems.length - 1]);
  });

  it("dismisses on Escape", async () => {
    const onClose = vi.fn();
    const { getByRole } = render(ContextMenu, {
      props: { x: 40, y: 40, items, onSelect: vi.fn(), onClose },
    });
    await tick();
    await fireEvent.keyDown(getByRole("menu"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("dismisses on an outside pointerdown but not an inside one", async () => {
    const onClose = vi.fn();
    const { getByRole } = render(ContextMenu, {
      props: { x: 40, y: 40, items, onSelect: vi.fn(), onClose },
    });
    await tick();
    await fireEvent.pointerDown(getByRole("menu"));
    expect(onClose).not.toHaveBeenCalled();
    await fireEvent.pointerDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("clamps the position so it never overflows the right/bottom viewport edge", async () => {
    const { getByRole } = render(ContextMenu, {
      props: { x: 1276, y: 716, items, onSelect: vi.fn(), onClose: vi.fn() },
    });
    await tick();
    await tick();
    const menu = getByRole("menu") as HTMLElement;
    const left = parseFloat(menu.style.left);
    const top = parseFloat(menu.style.top);
    expect(left).toBeLessThanOrEqual(window.innerWidth);
    expect(top).toBeLessThanOrEqual(window.innerHeight);
    expect(left).toBeGreaterThanOrEqual(0);
    expect(top).toBeGreaterThanOrEqual(0);
  });
});
