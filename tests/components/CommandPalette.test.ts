import { describe, it, expect, afterEach } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import CommandPalette from "@/components/CommandPalette.svelte";
import { commandPaletteOpen } from "@/lib/stores";

afterEach(() => commandPaletteOpen.set(false));

describe("CommandPalette popup (rendered)", () => {
  it("renders nothing while closed", () => {
    commandPaletteOpen.set(false);
    const { container } = render(CommandPalette);
    expect(container.querySelector(".palette")).toBeNull();
  });

  it("renders the spotlight input and category chips when opened", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.getAttribute("placeholder")).toContain("Search commands");
    const cats = Array.from(container.querySelectorAll(".category")).map((c) => c.textContent?.trim());
    expect(cats).toEqual(expect.arrayContaining(["All", "Navigate", "Action", "Settings"]));
  });

  it("lists navigation commands with category tags", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    expect(container.textContent).toContain("Go to Library");
    expect(container.textContent).toContain("Go to Catalog");
    expect(container.querySelectorAll(".result").length).toBeGreaterThan(0);
    const tags = Array.from(container.querySelectorAll(".result-tag")).map((t) => t.getAttribute("data-cat"));
    expect(tags).toContain("navigate");
  });

  it("filters results as the query narrows", async () => {
    commandPaletteOpen.set(true);
    const { container } = render(CommandPalette);
    await tick();
    const input = container.querySelector(".palette-search input") as HTMLInputElement;
    input.value = "backups";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    expect(container.textContent).toContain("Go to Backups");
    expect(container.textContent).not.toContain("Go to Catalog");
  });
});
