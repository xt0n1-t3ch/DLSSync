import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
// v1.6.7 decomposed the drawer: hero/back/learn-more → DrawerHero, the KPI
// summary → DrawerFeatureList, the action bar → DrawerFooter; the orchestrator
// (GameDetailDrawer) keeps the in-flow layout + Escape + feature-list logic.
const drawerSource = readFileSync(resolve(root, "components/GameDetailDrawer.svelte"), "utf8");
const heroSource = readFileSync(resolve(root, "components/DrawerHero.svelte"), "utf8");
const featureSource = readFileSync(resolve(root, "components/DrawerFeatureList.svelte"), "utf8");
const footerSource = readFileSync(resolve(root, "components/DrawerFooter.svelte"), "utf8");
const appSource = readFileSync(resolve(root, "App.svelte"), "utf8");
const librarySource = readFileSync(resolve(root, "views/Library.svelte"), "utf8");
const uxSource = readFileSync(resolve(root, "lib/ux.ts"), "utf8");
const enCatalog = readFileSync(resolve(root, "lib/i18n/locales/en.json"), "utf8");

function countMatches(haystack: string, re: RegExp): number {
  return (haystack.match(re) ?? []).length;
}

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
    expect(heroSource).toMatch(/class="detail-back"[\s\S]*?onclick=\{onClose\}/);
    expect(enCatalog).toMatch(/Back to Library/);
    expect(drawerSource).toMatch(/e\.key === "Escape"\) onClose\(\)/);
  });

  it("uses a compact hero banner (fixed height, not a 16:9 art block) with the launcher-accent stripe", () => {
    expect(heroSource).toMatch(/\.detail-hero\s*\{/);
    const art = ruleBody(heroSource, ".drawer-art");
    expect(art).toMatch(/height:\s*clamp/);
    expect(art).not.toMatch(/aspect-ratio/);
    expect(heroSource).toMatch(/\.drawer-art::before\s*\{[\s\S]*?--launcher-accent/);
  });

  it("the KPI summary scrolls with the page (not sticky) and the action bar is sticky-bottom", () => {
    const summary = ruleBody(featureSource, ".summary-row");
    expect(summary).not.toMatch(/position:\s*sticky/);
    const foot = ruleBody(footerSource, ".drawer-foot");
    expect(foot).toMatch(/position:\s*sticky/);
    expect(foot).toMatch(/bottom:\s*0/);
  });

  it("aligns the warning-banner Learn-more anchor with the link-btn doctrine", () => {
    const learnMore = ruleBody(heroSource, ".learn-more");
    expect(learnMore).toMatch(/height:\s*28px/);
    expect(learnMore).toMatch(/border-radius:\s*var\(--radius-md\)/);
    expect(learnMore).not.toMatch(/text-transform:\s*uppercase/);
  });
});

describe("GameDetailView — wired at the app level (master-detail right rail)", () => {
  it("App renders the detail in a persistent right rail beside the library, not replacing it", () => {
    expect(appSource).toMatch(/import GameDetailDrawer from "\.\/components\/GameDetailDrawer\.svelte"/);
    expect(appSource).toMatch(/railGameId = \$derived\(\$currentView === "library" \? \$drawerGameId : null\)/);
    expect(appSource).toMatch(/\{#if railGameId\}/);
    expect(appSource).toMatch(/class="detail-rail"/);
    expect(appSource).toMatch(/class:has-rail=\{!!railGameId\}/);
    expect(appSource).toMatch(/var\(--rail-width\)/);
    expect(appSource).toContain("<GameDetailDrawer");
  });

  it("the library content stays mounted in its own primary column, never push-padded away", () => {
    expect(appSource).toMatch(/class="main-primary"/);
    expect(librarySource).not.toContain("GameDetailDrawer");
    expect(appSource).not.toContain("data-drawer-open");
    expect(appSource).not.toMatch(/padding-right:\s*var\(--drawer-width\)/);
  });
});

describe("GameDetailView — DLSS-Enabler Streamline copy + same-major offers (v1.6.x)", () => {
  it("centralizes the enabler banner copy in lib/ux.ts and drops the old 'Managed by Enabler' chip label", () => {
    expect(uxSource).not.toMatch(/ENABLER_MANAGED_LABEL/);
    expect(uxSource).toMatch(/export const ENABLER_MANAGED_NOTE =\s*\n?\s*"DLSS Enabler requires NVIDIA Streamline 2\.11 or newer/);
    expect(uxSource).toMatch(/same major version/);
  });

  it("shows the enabler banner from the centralized i18n note, never inlining the literal", () => {
    expect(drawerSource).toMatch(/import \{[^}]*\bisStreamlinePlugin\b[^}]*\} from "\.\.\/lib\/relation"/);
    expect(heroSource).toMatch(/\{#if dlssEnabler\}[\s\S]*?\$t\(["']note\.enablerManaged["']\)/);
    expect(enCatalog).toMatch(/DLSS Enabler requires NVIDIA Streamline 2\.11 or newer/);
    expect(heroSource).not.toMatch(/"Managed by Enabler"/);
  });

  it("no longer suppresses sl.* under an enabler — the per-row enabler treatment is gone", () => {
    expect(drawerSource).not.toMatch(/enablerManagedSl/);
    expect(drawerSource).not.toMatch(/ENABLER_MANAGED_LABEL/);
    expect(countMatches(drawerSource, /\{@const em = /g)).toBe(0);
    expect(countMatches(drawerSource, /\{#if em\}/g)).toBe(0);
  });

  it("both row checkboxes disable only on family-disabled / same / no-target (no enabler clause)", () => {
    expect(
      countMatches(featureSource, /disabled=\{fd\s*\|\|\s*rel\s*===\s*"same"\s*\|\|\s*rel\s*===\s*"no-target"\}/g),
    ).toBe(2);
  });

  it("the Streamline set members are gated on outdated sl.* only, so same-major lights up under an enabler", () => {
    expect(drawerSource).toMatch(
      /streamlineSetMembers\s*=\s*\$derived\([\s\S]*?isStreamlinePlugin\(filenameFromPath\(r\.path\)\) && isOutdated\(r\)\)/,
    );
    expect(drawerSource).not.toMatch(/&& !enablerManagedSl\(r\)/);
  });

  it("annotates the Update Streamline set action with the centralized override note", () => {
    expect(footerSource).toMatch(/title=\{`[^`]*\$\{STREAMLINE_OVERRIDE_NOTE\}`\}/);
  });
});
