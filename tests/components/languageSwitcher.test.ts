import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { get } from "svelte/store";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { render, fireEvent } from "@testing-library/svelte";
import Sidebar from "@/components/Sidebar.svelte";
import LanguageMenu from "@/components/LanguageMenu.svelte";
import * as stores from "@/lib/stores";
import { settings, languageMenuOpen } from "@/lib/stores";
import { locale, LOCALES, LOCALE_LABELS, setLocale } from "@/lib/i18n/index";
import type { AppSettings } from "@/lib/api";

const here = dirname(fileURLToPath(import.meta.url));
const readSrc = (rel: string): string =>
  readFileSync(resolve(here, "../../frontend/src", rel), "utf8");

const MINIMAL_SETTINGS: AppSettings = {
  launcher_overrides: {
    steam: [],
    epic: [],
    gog: [],
    ubisoft: [],
    ea_desktop: [],
    xbox: [],
    battlenet: [],
    custom: [],
  },
  update_prefs: {
    update_dlss: true,
    update_dlss_fg: true,
    update_dlss_rr: true,
    update_streamline: false,
    update_reflex: true,
    update_xess: true,
    update_fsr: true,
    update_direct_storage: true,
    create_backups: true,
    auto_apply_all_on_rescan: false,
  },
  ui_prefs: {
    theme: "dark",
    sidebar_collapsed: false,
    grid_density: "comfy",
    sort_order: "default",
    launcher_filter: "all",
    status_filter: "all",
    library_view_mode: "grid",
    library_density: "comfy",
    library_sort: "default",
    backups_group_by: "game",
    settings_active_tab: "general",
    command_palette_recent: [],
    show_support_nudge: true,
    language: "en",
  },
  steam_api: { api_key: "", steam_id: "" },
  steamgriddb: { api_key: "" },
  window_state: { width: null, height: null, top: null, left: null, maximized: false },
  blacklist: [],
  ignored: [],
  game_preferences: {},
  advanced: {
    dlss_debug_overlay: false,
    verbose_logs: false,
    allow_unsigned_dlls: false,
    prefer_stable_channel: true,
    apply_concurrency: 2,
  },
  network: {
    retry_attempts: 3,
    download_cache_ttl_secs: 86400,
    connect_timeout_secs: 30,
    chunk_timeout_secs: 60,
  },
} as unknown as AppSettings;

beforeEach(() => {
  settings.set(MINIMAL_SETTINGS);
  languageMenuOpen.set(false);
  setLocale("en");
});

afterEach(() => {
  settings.set(null);
  languageMenuOpen.set(false);
  setLocale("en");
});

describe("Sidebar — language pill", () => {
  it("renders the [data-language-toggle] button", async () => {
    const { container } = render(Sidebar);
    await tick();
    const btn = container.querySelector("[data-language-toggle]");
    expect(btn).not.toBeNull();
  });

  it("clicking the pill toggles languageMenuOpen store from false to true", async () => {
    expect(get(languageMenuOpen)).toBe(false);
    const { container } = render(Sidebar);
    await tick();
    const btn = container.querySelector<HTMLButtonElement>("[data-language-toggle]");
    expect(btn).not.toBeNull();
    await fireEvent.click(btn!);
    expect(get(languageMenuOpen)).toBe(true);
  });

  it("clicking the pill a second time toggles languageMenuOpen store back to false", async () => {
    const { container } = render(Sidebar);
    await tick();
    const btn = container.querySelector<HTMLButtonElement>("[data-language-toggle]");
    await fireEvent.click(btn!);
    expect(get(languageMenuOpen)).toBe(true);
    await fireEvent.click(btn!);
    expect(get(languageMenuOpen)).toBe(false);
  });
});

describe("LanguageMenu — open=true renders all locales", () => {
  it("renders one option per LOCALES entry when open", async () => {
    const { container } = render(LanguageMenu, {
      props: { open: true, onClose: vi.fn() },
    });
    await tick();
    const opts = container.querySelectorAll("[role='option']");
    expect(opts.length).toBe(LOCALES.length);
  });

  it("marks only the active locale as aria-selected=true", async () => {
    setLocale("en");
    const { container } = render(LanguageMenu, {
      props: { open: true, onClose: vi.fn() },
    });
    await tick();
    const opts = Array.from(container.querySelectorAll<HTMLElement>("[role='option']"));
    const selected = opts.filter((o) => o.getAttribute("aria-selected") === "true");
    const notSelected = opts.filter((o) => o.getAttribute("aria-selected") === "false");
    expect(selected.length).toBe(1);
    expect(notSelected.length).toBe(LOCALES.length - 1);
    expect(selected[0].textContent).toContain(LOCALE_LABELS["en"]);
  });

  it("renders nothing when closed", () => {
    const { container } = render(LanguageMenu, {
      props: { open: false, onClose: vi.fn() },
    });
    expect(container.querySelector(".lang-menu")).toBeNull();
  });

  it("each option displays the endonym from LOCALE_LABELS", async () => {
    const { container } = render(LanguageMenu, {
      props: { open: true, onClose: vi.fn() },
    });
    await tick();
    const opts = container.querySelectorAll("[role='option']");
    const texts = Array.from(opts).map((o) => o.textContent ?? "");
    for (const loc of LOCALES) {
      expect(texts.some((t) => t.includes(LOCALE_LABELS[loc]))).toBe(true);
    }
  });
});

describe("LanguageMenu — choosing a locale", () => {
  it("choosing a non-active locale updates the locale store", async () => {
    setLocale("en");
    const onClose = vi.fn();
    const { container } = render(LanguageMenu, { props: { open: true, onClose } });
    await tick();
    const opts = Array.from(container.querySelectorAll<HTMLButtonElement>("[role='option']"));
    const esOpt = opts.find((o) => o.textContent?.includes(LOCALE_LABELS["es"]));
    expect(esOpt).not.toBeUndefined();
    await fireEvent.click(esOpt!);
    expect(get(locale)).toBe("es");
  });

  it("choosing a locale calls persistSettings with the new language when settings is seeded", async () => {
    const spy = vi.spyOn(stores, "persistSettings");
    spy.mockResolvedValue(undefined);

    setLocale("en");
    settings.set(MINIMAL_SETTINGS);
    const onClose = vi.fn();
    const { container } = render(LanguageMenu, { props: { open: true, onClose } });
    await tick();

    const opts = Array.from(container.querySelectorAll<HTMLButtonElement>("[role='option']"));
    const esOpt = opts.find((o) => o.textContent?.includes(LOCALE_LABELS["es"]));
    await fireEvent.click(esOpt!);
    await tick();

    expect(spy).toHaveBeenCalled();
    const calledWith = spy.mock.calls[spy.mock.calls.length - 1][0] as AppSettings;
    expect(calledWith.ui_prefs.language).toBe("es");
    spy.mockRestore();
  });

  it("choosing a locale calls onClose", async () => {
    setLocale("en");
    const onClose = vi.fn();
    const { container } = render(LanguageMenu, { props: { open: true, onClose } });
    await tick();
    const opts = Array.from(container.querySelectorAll<HTMLButtonElement>("[role='option']"));
    const esOpt = opts.find((o) => o.textContent?.includes(LOCALE_LABELS["es"]));
    await fireEvent.click(esOpt!);
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("Mount-location contract (backdrop-root rule)", () => {
  it("App.svelte imports and renders LanguageMenu at the root level", () => {
    const app = readSrc("App.svelte");
    expect(app).toContain("import LanguageMenu");
    expect(app).toContain("<LanguageMenu");
    expect(app).toContain("languageMenuOpen");
  });

  it("Sidebar.svelte does NOT nest LanguageMenu (trigger only flips the store)", () => {
    const sidebar = readSrc("components/Sidebar.svelte");
    expect(sidebar).not.toContain("<LanguageMenu");
    expect(sidebar).not.toContain("import LanguageMenu");
  });

  it("Sidebar.svelte carries data-language-toggle on the pill trigger", () => {
    const sidebar = readSrc("components/Sidebar.svelte");
    expect(sidebar).toContain("data-language-toggle");
  });

  it("LanguageMenu is positioned fixed (not absolute inside a glass ancestor)", () => {
    const src = readSrc("components/LanguageMenu.svelte");
    expect(src).toMatch(/\.lang-menu\s*\{[^}]*position:\s*fixed/);
    expect(src).not.toMatch(/\.lang-menu\s*\{[^}]*position:\s*absolute/);
  });
});
