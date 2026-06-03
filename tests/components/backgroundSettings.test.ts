import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { render, fireEvent, cleanup } from "@testing-library/svelte";
import { DEFAULT_BACKGROUND_CONFIG, type AppSettings } from "@/lib/api";

function fullSettings(over: Partial<AppSettings> = {}): AppSettings {
  return {
    launcher_overrides: { steam: [], epic: [], gog: [], ubisoft: [], ea_desktop: [], xbox: [], battlenet: [], custom: [] },
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
      prefer_stable_channel: false,
      apply_concurrency: 2,
    },
    network: { retry_attempts: 3, download_cache_ttl_secs: 300, connect_timeout_secs: 10, chunk_timeout_secs: 60 },
    background: { ...DEFAULT_BACKGROUND_CONFIG },
    ...over,
  };
}

const saveSettingsSpy = vi.fn(async () => undefined);

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  const appPaths = {
    root: "C:\\Users\\me\\DLSSync",
    backups_dir: "",
    cache_dir: "",
    logs_dir: "",
    settings_dir: "",
    backups_db: "",
    catalog_cache: "",
    settings_file: "C:\\Users\\me\\DLSSync\\settings.json",
  };
  return {
    ...actual,
    getSettings: vi.fn(async () => fullSettings()),
    saveSettings: (s: AppSettings) => saveSettingsSpy(s),
    getAppPaths: vi.fn(async () => appPaths),
    getDlssDebugOverlay: vi.fn(async () => false),
    checkDriverUpdates: vi.fn(async () => []),
  };
});

import Settings from "@/views/Settings.svelte";
import { settings } from "@/lib/stores";

beforeEach(() => {
  saveSettingsSpy.mockClear();
});

afterEach(() => {
  cleanup();
  settings.set(null);
});

async function settle(): Promise<void> {
  await tick();
  await Promise.resolve();
  await Promise.resolve();
  await tick();
  await tick();
}

describe("Settings — Background updates section", () => {
  it("renders the daemon section with all five toggles and the interval select", async () => {
    const { getByText, container } = render(Settings, { props: { onToggleTheme: vi.fn(), currentTheme: "dark" } });
    await settle();

    expect(getByText("Background updates")).toBeTruthy();
    expect(getByText("Enable background scanning")).toBeTruthy();
    expect(getByText("Close to tray instead of quitting")).toBeTruthy();
    expect(getByText("Start with Windows (minimized to tray)")).toBeTruthy();
    expect(getByText("Windows notification when updates are ready")).toBeTruthy();
    expect(getByText("Auto-apply updates")).toBeTruthy();
    expect(container.querySelector("#bg-interval")).not.toBeNull();
  });

  it("offers interval presets clamped within 1..168 hours", async () => {
    const { container } = render(Settings, { props: { onToggleTheme: vi.fn(), currentTheme: "dark" } });
    await settle();
    const select = container.querySelector("#bg-interval") as HTMLSelectElement;
    const values = Array.from(select.options).map((o) => Number(o.value));
    expect(values.length).toBeGreaterThan(0);
    expect(Math.min(...values)).toBeGreaterThanOrEqual(1);
    expect(Math.max(...values)).toBeLessThanOrEqual(168);
    expect(values).toContain(24);
  });

  it("persists background.enabled when the master toggle is flipped on", async () => {
    const { getByText } = render(Settings, { props: { onToggleTheme: vi.fn(), currentTheme: "dark" } });
    await settle();

    const row = getByText("Enable background scanning").closest(".row") as HTMLElement;
    const checkbox = row.querySelector('input[type="checkbox"]') as HTMLInputElement;
    expect(checkbox.checked).toBe(false);

    await fireEvent.change(checkbox, { target: { checked: true } });
    await settle();

    expect(saveSettingsSpy).toHaveBeenCalled();
    const saved = saveSettingsSpy.mock.calls.at(-1)![0] as AppSettings;
    expect(saved.background.enabled).toBe(true);
  });

  it("persists the chosen scan interval", async () => {
    const { container } = render(Settings, { props: { onToggleTheme: vi.fn(), currentTheme: "dark" } });
    await settle();
    const select = container.querySelector("#bg-interval") as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "48" } });
    await settle();

    expect(saveSettingsSpy).toHaveBeenCalled();
    const saved = saveSettingsSpy.mock.calls.at(-1)![0] as AppSettings;
    expect(saved.background.interval_hours).toBe(48);
  });
});
