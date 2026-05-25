import { describe, it, expect, afterEach } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import ShortcutOverlay from "@/components/ShortcutOverlay.svelte";
import { shortcutOverlayOpen } from "@/lib/stores";

afterEach(() => shortcutOverlayOpen.set(false));

describe("ShortcutOverlay popup (rendered)", () => {
  it("renders nothing while closed", () => {
    shortcutOverlayOpen.set(false);
    const { container } = render(ShortcutOverlay);
    expect(container.querySelector(".overlay-backdrop")).toBeNull();
  });

  it("renders the shortcuts dialog when opened", async () => {
    shortcutOverlayOpen.set(true);
    const { container } = render(ShortcutOverlay);
    await tick();
    const dialog = container.querySelector('[role="dialog"][aria-label="Keyboard shortcuts"]');
    expect(dialog).not.toBeNull();
    expect(container.textContent).toContain("Keyboard shortcuts");
    expect(container.querySelectorAll(".group").length).toBeGreaterThan(0);
    expect(container.querySelectorAll(".kbd").length).toBeGreaterThan(0);
  });

  it("close button collapses the overlay via the store", async () => {
    shortcutOverlayOpen.set(true);
    const { container } = render(ShortcutOverlay);
    await tick();
    const close = container.querySelector('button[aria-label="Close shortcuts overlay"]') as HTMLButtonElement;
    expect(close).not.toBeNull();
    close.click();
    await tick();
    expect(container.querySelector(".overlay-backdrop")).toBeNull();
  });
});
