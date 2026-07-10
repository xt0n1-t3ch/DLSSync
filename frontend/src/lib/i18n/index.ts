import { derived, writable, type Readable, type Writable } from "svelte/store";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type Messages = typeof en;
export type Locale = "en" | "es" | "pt-BR" | "de" | "fr" | "ja" | "ru" | "zh-CN";
export type TranslationVars = Record<string, string | number>;

export const LOCALES: readonly Locale[] = ["en", "es", "pt-BR", "de", "fr", "ja", "ru", "zh-CN"];
export const DEFAULT_LOCALE: Locale = "en";
export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  es: "Español",
  "pt-BR": "Português (Brasil)",
  de: "Deutsch",
  fr: "Français",
  ja: "日本語",
  ru: "Русский",
  "zh-CN": "简体中文",
};

const CATALOGS: Partial<Record<Locale, Messages>> = { en, es };
const CATALOG_LOADERS: Record<Locale, () => Promise<Messages>> = {
  en: async () => en,
  es: async () => es,
  "pt-BR": async () => (await import("./locales/pt-BR.json")).default as Messages,
  de: async () => (await import("./locales/de.json")).default as Messages,
  fr: async () => (await import("./locales/fr.json")).default as Messages,
  ja: async () => (await import("./locales/ja.json")).default as Messages,
  ru: async () => (await import("./locales/ru.json")).default as Messages,
  "zh-CN": async () => (await import("./locales/zh-CN.json")).default as Messages,
};
const pendingCatalogs = new Map<Locale, Promise<Messages>>();

export const locale: Writable<Locale> = writable(DEFAULT_LOCALE);

export function isLocale(value: string | null | undefined): value is Locale {
  return value != null && (LOCALES as readonly string[]).includes(value);
}

export function localeFromNavigator(): Locale {
  const tag = typeof navigator === "undefined" ? "" : navigator.language.toLowerCase();
  if (tag.startsWith("es")) return "es";
  if (tag.startsWith("pt")) return "pt-BR";
  if (tag.startsWith("de")) return "de";
  if (tag.startsWith("fr")) return "fr";
  if (tag.startsWith("ja")) return "ja";
  if (tag.startsWith("ru")) return "ru";
  if (tag.startsWith("zh")) return "zh-CN";
  return DEFAULT_LOCALE;
}

export function setLocale(next: Locale): void {
  if (CATALOGS[next] === undefined) {
    void loadLocale(next);
    return;
  }
  locale.set(next);
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("lang", next);
  }
}

export async function loadLocale(next: Locale): Promise<void> {
  let catalog = CATALOGS[next];
  if (catalog === undefined) {
    let pending = pendingCatalogs.get(next);
    if (pending === undefined) {
      pending = CATALOG_LOADERS[next]();
      pendingCatalogs.set(next, pending);
    }
    try {
      catalog = await pending;
      CATALOGS[next] = catalog;
    } finally {
      pendingCatalogs.delete(next);
    }
  }
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
  const fallback = CATALOGS[DEFAULT_LOCALE]!;
  const active = CATALOGS[loc] ?? fallback;
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
