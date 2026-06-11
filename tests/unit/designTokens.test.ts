import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(
  resolve(here, "../../frontend/src/styles/global.css"),
  "utf8",
);

function ruleBody(source: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`${escaped}\\s*\\{([^}]*)\\}`).exec(source)?.[1] ?? "";
}

describe("design tokens — spacing scale, density, tactile + glass-dialog utilities", () => {
  it("defines the 8pt spacing scale --space-1..8 with the canonical step values", () => {
    const steps: Record<string, string> = {
      "--space-1": "4px",
      "--space-2": "8px",
      "--space-3": "12px",
      "--space-4": "16px",
      "--space-5": "24px",
      "--space-6": "32px",
      "--space-7": "48px",
      "--space-8": "64px",
    };
    for (const [token, value] of Object.entries(steps)) {
      expect(css).toMatch(new RegExp(`${token}:\\s*${value};`));
    }
  });

  it("exposes a density multiplier hook", () => {
    expect(css).toMatch(/--density:\s*1;/);
  });

  it("defines the master-detail right-rail width token", () => {
    expect(css).toMatch(/--rail-width:\s*clamp\([^;]+\);/);
  });

  it("provides the shared tactile utilities .hover-lift and .press", () => {
    expect(ruleBody(css, ".hover-lift:hover")).toMatch(
      /translateY\(-2px\)\s*scale\(1\.02\)/,
    );
    expect(ruleBody(css, ".press:active")).toMatch(/scale\(0\.98\)/);
  });

  it("provides one centralized .glass-dialog util (blur + shadow + 3px stripe + 32px close)", () => {
    const dialog = ruleBody(css, ".glass-dialog");
    expect(dialog).toMatch(/backdrop-filter:\s*var\(--glass-blur\)/);
    expect(dialog).toMatch(/box-shadow:[^;]*var\(--shadow-lg\)/);
    expect(css).toMatch(/\.glass-dialog::before\s*\{[^}]*width:\s*3px/);
    const close = ruleBody(css, ".dialog-close");
    expect(close).toMatch(/width:\s*32px/);
    expect(close).toMatch(/height:\s*32px/);
  });

  it("keeps a backdrop-filter fallback for .glass-dialog", () => {
    expect(css).toMatch(
      /\.glass-dialog\s*\{\s*background:\s*var\(--glass-fallback\)/,
    );
  });

  it("centralizes tokenized scrollbar styling with forced-colors fallback", () => {
    expect(css).toMatch(/--scrollbar-thumb:\s*rgba\(/);
    expect(css).toMatch(/--scrollbar-thumb-hover:\s*rgba\(/);
    expect(css).toMatch(/::-webkit-scrollbar-thumb\s*\{[^}]*var\(--scrollbar-thumb\)/);
    expect(css).toMatch(/@supports not selector\(::-webkit-scrollbar\)/);
    expect(css).toMatch(/@media \(forced-colors:\s*active\)/);
  });
});
