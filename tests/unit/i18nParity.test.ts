import { describe, expect, it } from "vitest";
import en from "@/lib/i18n/locales/en.json";
import es from "@/lib/i18n/locales/es.json";
import meta from "@/lib/i18n/locales/_meta.json";

type JsonObject = Record<string, unknown>;
const CATALOGS: Record<string, JsonObject> = { en: en as JsonObject, es: es as JsonObject };
const PLURAL_SUFFIXES = ["_zero", "_one", "_two", "_few", "_many", "_other"];

function flattenKeys(value: unknown, prefix = ""): string[] {
  if (value === null || typeof value !== "object") return [prefix];
  return Object.entries(value as JsonObject).flatMap(([key, child]) =>
    flattenKeys(child, prefix ? `${prefix}.${key}` : key),
  );
}

function valueAt(catalog: JsonObject, path: string): unknown {
  return path
    .split(".")
    .reduce<unknown>(
      (acc, key) => (acc && typeof acc === "object" ? (acc as JsonObject)[key] : undefined),
      catalog,
    );
}

describe("i18n catalog parity", () => {
  const enKeys = flattenKeys(en);
  const enKeySet = new Set(enKeys);

  for (const [name, catalog] of Object.entries(CATALOGS)) {
    if (name === "en") continue;
    it(`${name}.json has exactly the same keys as en.json`, () => {
      const keys = new Set(flattenKeys(catalog));
      const missing = enKeys.filter((key) => !keys.has(key));
      const extra = [...keys].filter((key) => !enKeySet.has(key));
      expect(missing, `missing keys in ${name}.json`).toEqual([]);
      expect(extra, `extra keys in ${name}.json (not present in en.json)`).toEqual([]);
    });
  }

  it("every catalog value is a non-empty string", () => {
    for (const [name, catalog] of Object.entries(CATALOGS)) {
      for (const key of flattenKeys(catalog)) {
        const value = valueAt(catalog, key);
        const ok = typeof value === "string" && value.trim().length > 0;
        expect(ok, `${name}.json: ${key} must be a non-empty string`).toBe(true);
      }
    }
  });

  it("every _meta key maps to a real en.json key (plain or pluralised)", () => {
    for (const metaKey of Object.keys(meta as JsonObject)) {
      const covered =
        enKeySet.has(metaKey) || PLURAL_SUFFIXES.some((suffix) => enKeySet.has(`${metaKey}${suffix}`));
      expect(covered, `_meta.json describes a key absent from en.json: ${metaKey}`).toBe(true);
    }
  });
});
