import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";

const SRC_ROOT = resolve(__dirname, "../../frontend/src");
const RAW_HTML_ALLOWLIST: ReadonlySet<string> = new Set<string>();

function svelteFiles(): string[] {
  return readdirSync(SRC_ROOT, { recursive: true, encoding: "utf8" })
    .filter((rel) => rel.endsWith(".svelte"))
    .map((rel) => rel.replace(/\\/g, "/"));
}

describe("unsafe HTML sinks", () => {
  it("no component renders raw HTML outside the explicit allowlist", () => {
    const offenders = svelteFiles()
      .filter((rel) => !RAW_HTML_ALLOWLIST.has(rel))
      .filter((rel) => readFileSync(join(SRC_ROOT, rel), "utf8").includes("{@html"));
    expect(
      offenders,
      `{@html} found in: ${offenders.join(", ")} — sanitize upstream or add to RAW_HTML_ALLOWLIST deliberately`,
    ).toEqual([]);
  });

  it("scans a real component surface", () => {
    expect(svelteFiles().length).toBeGreaterThan(20);
  });

  it("locale strings never carry HTML tags for interpolated counts", () => {
    const en = readFileSync(join(SRC_ROOT, "lib/i18n/locales/en.json"), "utf8");
    const es = readFileSync(join(SRC_ROOT, "lib/i18n/locales/es.json"), "utf8");
    expect(en).not.toContain("<strong>{count}</strong>");
    expect(es).not.toContain("<strong>{count}</strong>");
  });
});
