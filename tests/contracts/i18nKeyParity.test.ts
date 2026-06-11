import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";

const SRC_ROOT = resolve(__dirname, "../../frontend/src");
const LOCALES_DIR = join(SRC_ROOT, "lib", "i18n", "locales");
const PLURAL_SUFFIXES = ["_zero", "_one", "_two", "_few", "_many", "_other"];
const STATIC_KEY = /^[a-z][\w-]*(\.[\w-]+)+$/i;
const REF_PATTERNS = [/\$t\(\s*["']([^"'{}]+)["']/g, /\btranslate\(\s*[^,]+,\s*["']([^"'{}]+)["']/g];

function flattenKeys(node: unknown, prefix: string, out: Set<string>): void {
  if (typeof node === "string") {
    out.add(prefix);
    return;
  }
  if (node !== null && typeof node === "object") {
    for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
      flattenKeys(value, prefix ? `${prefix}.${key}` : key, out);
    }
  }
}

function loadLocaleKeys(file: string): Set<string> {
  const keys = new Set<string>();
  flattenKeys(JSON.parse(readFileSync(join(LOCALES_DIR, file), "utf8")), "", keys);
  return keys;
}

function sourceFiles(): string[] {
  return readdirSync(SRC_ROOT, { recursive: true, encoding: "utf8" })
    .filter((rel) => /\.(svelte|ts)$/.test(rel) && !rel.includes("locales"))
    .map((rel) => join(SRC_ROOT, rel));
}

function referencedKeys(): Map<string, string[]> {
  const refs = new Map<string, string[]>();
  for (const file of sourceFiles()) {
    const source = readFileSync(file, "utf8");
    for (const pattern of REF_PATTERNS) {
      for (const match of source.matchAll(pattern)) {
        const key = match[1];
        if (!STATIC_KEY.test(key)) continue;
        const sites = refs.get(key) ?? [];
        sites.push(file);
        refs.set(key, sites);
      }
    }
  }
  return refs;
}

function resolves(key: string, catalog: Set<string>): boolean {
  return catalog.has(key) || PLURAL_SUFFIXES.some((suffix) => catalog.has(`${key}${suffix}`));
}

const enKeys = loadLocaleKeys("en.json");
const esKeys = loadLocaleKeys("es.json");
const refs = referencedKeys();

describe("i18n key parity contract", () => {
  it("resolves every statically referenced key in en.json", () => {
    const missing = [...refs.keys()].filter((key) => !resolves(key, enKeys)).sort();
    expect(missing).toEqual([]);
  });

  it("resolves every statically referenced key in es.json", () => {
    const missing = [...refs.keys()].filter((key) => !resolves(key, esKeys)).sort();
    expect(missing).toEqual([]);
  });

  it("defines the exact same key set in en.json and es.json", () => {
    const onlyEn = [...enKeys].filter((key) => !esKeys.has(key)).sort();
    const onlyEs = [...esKeys].filter((key) => !enKeys.has(key)).sort();
    expect({ onlyEn, onlyEs }).toEqual({ onlyEn: [], onlyEs: [] });
  });

  it("scans a non-trivial reference surface", () => {
    expect(refs.size).toBeGreaterThan(200);
  });
});
