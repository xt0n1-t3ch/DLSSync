import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
const drawerSource = readFileSync(resolve(root, "components/GameDetailDrawer.svelte"), "utf8");
const appSource = readFileSync(resolve(root, "App.svelte"), "utf8");
const librarySource = readFileSync(resolve(root, "views/Library.svelte"), "utf8");

function ruleBody(css: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`);
  return re.exec(css)?.[1] ?? "";
}

describe("GameDetailView — full-page detail, not an overlay drawer", () => {
  it("renders in-flow (no scrim, no fixed/overlay drawer, no modal attrs)", () => {
    expect(drawerSource).not.toContain("drawer-scrim");
    expect(drawerSource).not.toMatch(/\.drawer\s*\{[^}]*position:\s*fixed/);
    expect(drawerSource).not.toMatch(/role="dialog"/);
    expect(drawerSource).not.toMatch(/aria-modal/);
    expect(drawerSource).not.toMatch(/function trapFocus/);
    expect(drawerSource).not.toMatch(/matchMedia/);
  });

  it("has a Back-to-Library affordance wired to onClose, and Escape closes", () => {
    expect(drawerSource).toMatch(/class="detail-back"[\s\S]*?onclick=\{onClose\}/);
    expect(drawerSource).toMatch(/Back to Library/);
    expect(drawerSource).toMatch(/e\.key === "Escape"\) onClose\(\)/);
  });

  it("uses a compact hero banner (fixed height, not a 16:9 art block) with the launcher-accent stripe", () => {
    expect(drawerSource).toMatch(/\.detail-hero\s*\{/);
    const art = ruleBody(drawerSource, ".drawer-art");
    expect(art).toMatch(/height:\s*clamp/);
    expect(art).not.toMatch(/aspect-ratio/);
    expect(drawerSource).toMatch(/\.drawer-art::before\s*\{[\s\S]*?--launcher-accent/);
  });

  it("the KPI summary scrolls with the page (not sticky) and the action bar is sticky-bottom", () => {
    const summary = ruleBody(drawerSource, ".summary-row");
    expect(summary).not.toMatch(/position:\s*sticky/);
    const foot = ruleBody(drawerSource, ".drawer-foot");
    expect(foot).toMatch(/position:\s*sticky/);
    expect(foot).toMatch(/bottom:\s*16px/);
  });

  it("aligns the warning-banner Learn-more anchor with the link-btn doctrine", () => {
    const learnMore = ruleBody(drawerSource, ".learn-more");
    expect(learnMore).toMatch(/height:\s*28px/);
    expect(learnMore).toMatch(/border-radius:\s*var\(--radius-md\)/);
    expect(learnMore).not.toMatch(/text-transform:\s*uppercase/);
  });
});

describe("GameDetailView — wired at the app level (replaces the library content)", () => {
  it("App renders the detail in the content area when a game is open, not Library as an overlay", () => {
    expect(appSource).toMatch(/import GameDetailDrawer from "\.\/components\/GameDetailDrawer\.svelte"/);
    expect(appSource).toMatch(/\$currentView === "library" && \$drawerGameId/);
    expect(appSource).toContain("<GameDetailDrawer");
  });

  it("Library no longer renders the detail itself and drops the push-padding side-panel", () => {
    expect(librarySource).not.toContain("GameDetailDrawer");
    expect(appSource).not.toContain("data-drawer-open");
    expect(appSource).not.toMatch(/padding-right:\s*var\(--drawer-width\)/);
  });
});
