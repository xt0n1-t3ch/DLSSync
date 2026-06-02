import { describe, expect, it } from "vitest";
import { get } from "svelte/store";
import {
  interpolate,
  isLocale,
  setLocale,
  t,
  translate,
  type Locale,
} from "@/lib/i18n/index";

describe("i18n engine", () => {
  it("interpolates named params", () => {
    expect(interpolate("Found {count} games", { count: 3 })).toBe("Found 3 games");
  });

  it("leaves an unfilled placeholder visible instead of undefined", () => {
    expect(interpolate("Hi {name}")).toBe("Hi {name}");
    expect(interpolate("Hi {name}", { other: "x" })).toBe("Hi {name}");
  });

  it("resolves a plain key", () => {
    expect(translate("en", "app.name")).toBe("DLSSync");
    expect(translate("es", "language.label")).toBe("Idioma");
  });

  it("selects English plural one/other", () => {
    expect(translate("en", "common.updatesReady", { count: 1 })).toBe("1 update ready");
    expect(translate("en", "common.updatesReady", { count: 5 })).toBe("5 updates ready");
  });

  it("selects Spanish plural one/other", () => {
    expect(translate("es", "common.updatesReady", { count: 1 })).toBe("1 actualización lista");
    expect(translate("es", "common.updatesReady", { count: 2 })).toBe("2 actualizaciones listas");
  });

  it("returns the raw path for an unknown key", () => {
    expect(translate("en", "does.not.exist")).toBe("does.not.exist");
  });

  it("falls back to the default locale for an unknown locale", () => {
    expect(translate("fr" as Locale, "app.name")).toBe("DLSSync");
  });

  it("narrows locale tags with isLocale", () => {
    expect(isLocale("en")).toBe(true);
    expect(isLocale("es")).toBe(true);
    expect(isLocale("")).toBe(false);
    expect(isLocale("fr")).toBe(false);
    expect(isLocale(null)).toBe(false);
  });

  it("re-derives the t store when the locale changes", () => {
    setLocale("en");
    expect(get(t)("language.label")).toBe("Language");
    setLocale("es");
    expect(get(t)("language.label")).toBe("Idioma");
    setLocale("en");
  });
});
