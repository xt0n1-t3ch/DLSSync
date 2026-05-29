import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const read = (rel: string): string => readFileSync(resolve(here, rel), "utf8");

const css = read("../../frontend/src/styles/global.css");
const gameCard = read("../../frontend/src/components/GameCard.svelte");
const gameListRow = read("../../frontend/src/components/GameListRow.svelte");

function ruleBody(src: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`);
  return re.exec(src)?.[1] ?? "";
}

describe("Phase 4 — flat opaque base material discipline", () => {
  it("base content-surface tokens are OPAQUE hex, never translucent rgba — dark AND light", () => {
    for (const token of ["--bg-card", "--bg-card-hover", "--bg-elevated"]) {
      expect(css).not.toMatch(new RegExp(`${token}:\\s*rgba`, "i"));
      const hexHits = css.match(new RegExp(`${token}:\\s*#[0-9a-f]{3,8}\\b`, "gi")) ?? [];
      expect(hexHits.length).toBeGreaterThanOrEqual(2);
    }
  });

  it("Library base-layer surfaces declare NO backdrop-filter on the card/row body", () => {
    expect(ruleBody(gameCard, ".game-card")).not.toMatch(/backdrop-filter/);
    expect(ruleBody(gameListRow, ".list-row")).not.toMatch(/backdrop-filter/);
  });

  it("content cards carry no at-rest shadow — structure from borders, not elevation", () => {
    expect(ruleBody(css, ".aura-card")).not.toMatch(/box-shadow\s*:/);
    expect(ruleBody(css, ".card")).not.toMatch(/box-shadow\s*:/);
  });

  it("glass stays reserved for floating chrome — .glass-panel keeps backdrop-filter with a fallback", () => {
    expect(ruleBody(css, ".glass-panel")).toMatch(/backdrop-filter/);
    expect(css).toMatch(/@supports not \(\(backdrop-filter/);
  });
});
