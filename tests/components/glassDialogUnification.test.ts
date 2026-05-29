import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const componentsDir = resolve(here, "../../frontend/src/components");

function source(name: string): string {
  return readFileSync(resolve(componentsDir, name), "utf8");
}

describe("Phase 8 — floating chrome shares the .glass-dialog material", () => {
  const dialogPanels = [
    "ApplyProgressModal.svelte",
    "VersionPickerPopover.svelte",
    "CatalogVersionsFlyout.svelte",
    "DriverHistoryFlyout.svelte",
    "Select.svelte",
    "NotificationsBell.svelte",
  ];

  it("applies the .glass-dialog class on every floating-chrome panel", () => {
    for (const file of dialogPanels) {
      expect(source(file), `${file} should carry glass-dialog`).toContain("glass-dialog");
    }
  });

  const closableDialogs = [
    "ApplyProgressModal.svelte",
    "VersionPickerPopover.svelte",
    "CatalogVersionsFlyout.svelte",
    "DriverHistoryFlyout.svelte",
  ];

  it("uses the shared .dialog-close affordance instead of bespoke close buttons", () => {
    for (const file of closableDialogs) {
      const src = source(file);
      expect(src, `${file} should use dialog-close`).toContain('class="dialog-close"');
      expect(src, `${file} must not keep a bespoke close class`).not.toMatch(
        /class="(modal-close|picker-close|flyout-close)"/,
      );
    }
  });

  it("drops the bespoke per-component glass/vendor-stripe surfaces", () => {
    for (const file of dialogPanels) {
      const src = source(file);
      expect(src, `${file} must not redefine backdrop-filter locally`).not.toContain(
        "backdrop-filter: var(--glass-blur)",
      );
    }
    expect(source("CatalogVersionsFlyout.svelte")).not.toContain('class="vendor-stripe"');
    expect(source("DriverHistoryFlyout.svelte")).not.toContain('class="vendor-stripe"');
  });

  it("routes vendor accent through the shared stripe via --edge-color", () => {
    expect(source("CatalogVersionsFlyout.svelte")).toContain("--edge-color={accent}");
    expect(source("DriverHistoryFlyout.svelte")).toContain("--edge-color={accent}");
  });
});
