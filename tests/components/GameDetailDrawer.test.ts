import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const drawerSource = readFileSync(
  resolve(here, "../../frontend/src/components/GameDetailDrawer.svelte"),
  "utf8",
);

function ruleBody(css: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`);
  return re.exec(css)?.[1] ?? "";
}

describe("GameDetailDrawer redesign — scrim, highlight, stripe, rhythm", () => {
  it("declares the scrim without backdrop-filter and with a radial-gradient base", () => {
    const scrimBlock = ruleBody(drawerSource, ".drawer-scrim");
    expect(scrimBlock).not.toMatch(/backdrop-filter/);
    expect(drawerSource).toMatch(/\.drawer-scrim\s*\{[\s\S]*?radial-gradient/);
    expect(drawerSource).toMatch(/@media\s*\(prefers-reduced-transparency: reduce\)/);
  });

  it("adds a Tahoe-style top-edge highlight on .drawer and a launcher-accent stripe on .drawer-art", () => {
    expect(drawerSource).toMatch(/\.drawer::before\s*\{[\s\S]*?linear-gradient/);
    expect(drawerSource).toMatch(/\.drawer-art::before\s*\{[\s\S]*?--launcher-accent/);
  });

  it("aligns the warning-banner Learn-more anchor with the link-btn doctrine", () => {
    const learnMore = ruleBody(drawerSource, ".learn-more");
    expect(learnMore).toMatch(/height:\s*28px/);
    expect(learnMore).toMatch(/border-radius:\s*var\(--radius-md\)/);
    expect(learnMore).not.toMatch(/text-transform:\s*uppercase/);
  });

  it("widens section rhythm: warning-banner 20px, summary-row 16px, advanced-block 16px", () => {
    const warning = ruleBody(drawerSource, ".warning-banner");
    expect(warning).toMatch(/margin-bottom:\s*20px/);
    const summary = ruleBody(drawerSource, ".summary-row");
    expect(summary).toMatch(/margin-bottom:\s*16px/);
    const advanced = ruleBody(drawerSource, ".advanced-block");
    expect(advanced).toMatch(/margin-top:\s*16px/);
  });
});
