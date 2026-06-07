import { describe, it, expect } from "vitest";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = resolve(here, "../../frontend/dist/assets");

const BUDGET_JS_GZIP_BYTES = 250 * 1024;
const BUDGET_CSS_GZIP_BYTES = 75 * 1024;
const KNOWN_CHUNK_PREFIXES = ["index-", "image-", "app-", "window-"];
const STEALTH_CHUNK_MIN_BYTES = 1024;

function gzipBytes(path: string): number {
  return gzipSync(readFileSync(path)).length;
}

function fmt(bytes: number): string {
  return `${(bytes / 1024).toFixed(1)} KB`;
}

describe.skipIf(!existsSync(distDir))(
  "bundle size budget (post-build, gzip-measured)",
  () => {
    it("the biggest index-*.js chunk stays under the gzip budget", () => {
      type Chunk = { name: string; gzip: number };
      const candidates: Chunk[] = readdirSync(distDir)
        .filter((f: string) => f.startsWith("index-") && f.endsWith(".js"))
        .map(
          (name: string): Chunk => ({
            name,
            gzip: gzipBytes(resolve(distDir, name)),
          }),
        )
        .sort((a: Chunk, b: Chunk) => b.gzip - a.gzip);

      expect(candidates.length, `no index-*.js in ${distDir}`).toBeGreaterThan(0);

      const biggest = candidates[0];
      expect(
        biggest.gzip,
        `${biggest.name} = ${fmt(biggest.gzip)} gzip exceeds the ${fmt(BUDGET_JS_GZIP_BYTES)} budget`,
      ).toBeLessThan(BUDGET_JS_GZIP_BYTES);
    });

    it("every index-*.css bundle stays under the gzip budget", () => {
      type Chunk = { name: string; gzip: number };
      const candidates: Chunk[] = readdirSync(distDir)
        .filter((f: string) => f.startsWith("index-") && f.endsWith(".css"))
        .map(
          (name: string): Chunk => ({
            name,
            gzip: gzipBytes(resolve(distDir, name)),
          }),
        );

      expect(candidates.length, `no index-*.css in ${distDir}`).toBeGreaterThan(0);

      for (const c of candidates) {
        expect(
          c.gzip,
          `${c.name} = ${fmt(c.gzip)} gzip exceeds the ${fmt(BUDGET_CSS_GZIP_BYTES)} budget`,
        ).toBeLessThan(BUDGET_CSS_GZIP_BYTES);
      }
    });

    it("no stealth JS chunk lands outside the known emit prefixes", () => {
      const surprises: string[] = readdirSync(distDir)
        .filter((f: string) => f.endsWith(".js"))
        .filter((f: string) => gzipBytes(resolve(distDir, f)) > STEALTH_CHUNK_MIN_BYTES)
        .filter((f: string) => !KNOWN_CHUNK_PREFIXES.some((p) => f.startsWith(p)));

      expect(
        surprises,
        `unknown >${fmt(STEALTH_CHUNK_MIN_BYTES)} JS chunks: ${surprises.join(", ")} — add the prefix to KNOWN_CHUNK_PREFIXES intentionally after reviewing what's inside`,
      ).toEqual([]);
    });
  },
);
