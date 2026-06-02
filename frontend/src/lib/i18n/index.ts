import { derived, writable, type Readable, type Writable } from "svelte/store";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type Messages = typeof en;
export type Locale = "en" | "es";
export type TranslationVars = Record<string, string | number>;

export const LOCALES: readonly Locale[] = ["en", "es"];
export const DEFAULT_LOCALE: Locale = "en";
export const LOCALE_LABELS: Record<Locale, string> = { en: "English", es: "Español" };

const CATALOGS: Record<Locale, Messages> = { en, es };

export const locale: Writable<Locale> = writable(DEFAULT_LOCALE);

export function isLocale(value: string | null | undefined): value is Locale {
  return value != null && (LOCALES as readonly string[]).includes(value);
}

export function localeFromNavigator(): Locale {
  const tag = typeof navigator === "undefined" ? "" : navigator.language;
  return tag.toLowerCase().startsWith("es") ? "es" : DEFAULT_LOCALE;
}

export function setLocale(next: Locale): void {
  locale.set(next);
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("lang", next);
  }
}

export function interpolate(template: string, vars?: TranslationVars): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_match, name: string) =>
    name in vars ? String(vars[name]) : `{${name}}`,
  );
}

const pluralRules = new Map<Locale, Intl.PluralRules>();

function pluralCategory(loc: Locale, count: number): string {
  let rules = pluralRules.get(loc);
  if (rules === undefined) {
    rules = new Intl.PluralRules(loc);
    pluralRules.set(loc, rules);
  }
  return rules.select(count);
}

function lookup(catalog: Messages, path: string): string | undefined {
  const node = path.split(".").reduce<unknown>(
    (acc, key) =>
      acc !== null && typeof acc === "object" ? (acc as Record<string, unknown>)[key] : undefined,
    catalog,
  );
  return typeof node === "string" ? node : undefined;
}

export function translate(loc: Locale, path: string, vars?: TranslationVars): string {
  const active = CATALOGS[loc] ?? CATALOGS[DEFAULT_LOCALE];
  const fallback = CATALOGS[DEFAULT_LOCALE];
  const count = vars?.count;
  const pluralPath =
    typeof count === "number" ? `${path}_${pluralCategory(loc, count)}` : null;
  const raw =
    (pluralPath !== null ? lookup(active, pluralPath) : undefined) ??
    (pluralPath !== null ? lookup(fallback, pluralPath) : undefined) ??
    lookup(active, path) ??
    lookup(fallback, path) ??
    path;
  return interpolate(raw, vars);
}

export const t: Readable<(path: string, vars?: TranslationVars) => string> = derived(
  locale,
  ($locale) =>
    (path: string, vars?: TranslationVars): string =>
      translate($locale, path, vars),
);
