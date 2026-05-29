import { describe, it, expect, afterEach } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import CommandPalette from "@/components/CommandPalette.svelte";
import { commandPaletteOpen } from "@/lib/stores";

afterEach(() => commandPaletteOpen.set(false));

function press(el: Element, key: string): void {
  el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

describe("CommandPalette popup (rendered)", () => {
  it("renders nothing while closed", () => {
    commandPaletteOpen.set(false);
    const { container } = render(CommandPalette);
    expect(container.querySelector(".palette")).toBeNull();
  });

  it("renders the boxed search field, an Esc chip and the category chips", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.getAttribute("placeholder")).toContain("Search commands");
    expect(container.querySelector(".palette-search-kbd")?.textContent).toContain("Esc");
    const cats = Array.from(container.querySelectorAll(".category")).map((c) => c.textContent?.trim());
    expect(cats).toEqual(expect.arrayContaining(["All", "Navigate", "Action", "Settings"]));
  });

  it("groups commands under section headers, each row with a category-tinted icon", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const heads = Array.from(container.querySelectorAll(".result-group-head")).map((h) => h.textContent?.trim());
    expect(heads).toEqual(expect.arrayContaining(["Navigate", "Action", "Settings"]));
    expect(container.textContent).toContain("Go to Library");
    expect(container.querySelectorAll(".result").length).toBeGreaterThan(0);
    const cats = Array.from(container.querySelectorAll(".result-icon")).map((i) => i.getAttribute("data-cat"));
    expect(cats).toContain("navigate");
  });

  it("renders a keyboard-shortcut chip for commands that declare one", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const library = Array.from(container.querySelectorAll(".result")).find((r) => r.textContent?.includes("Go to Library"));
    const keys = Array.from(library?.querySelectorAll(".result-shortcut kbd") ?? []).map((k) => k.textContent?.trim());
    expect(keys).toEqual(["G", "L"]);
  });

  it("filters and highlights the matched characters as the query narrows", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    input.value = "catalog";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    expect(container.textContent).toContain("Go to Catalog");
    expect(container.textContent).not.toContain("Go to About");
    const marks = Array.from(container.querySelectorAll(".result-hl")).map((m) => m.textContent);
    expect(marks).toContain("Catalog");
  });

  it("arrow keys move the active selection across group boundaries", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    const activeTitle = (): string | undefined =>
      container.querySelector(".result.active .result-title")?.textContent?.trim();
    expect(activeTitle()).toBe("Go to Library");
    press(input, "ArrowDown");
    await tick();
    expect(activeTitle()).toBe("Go to Catalog");
  });

  it("shows a rich empty state when nothing matches", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    input.value = "zzqzzq";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    expect(container.querySelector(".palette-empty")).not.toBeNull();
    expect(container.querySelector(".palette-empty-title")?.textContent).toContain("zzqzzq");
    expect(container.querySelectorAll(".result").length).toBe(0);
  });
});
