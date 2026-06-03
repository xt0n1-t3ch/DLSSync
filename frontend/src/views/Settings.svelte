<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import PerformanceToggles from "../components/PerformanceToggles.svelte";
  import {
    settings,
    persistSettings,
    loadSettings,
    showToast,
    scanGames,
    currentView,
    driverReports,
    loadDriverUpdates,
  } from "../lib/stores";
  import {
    setDlssDebugOverlay,
    getDlssDebugOverlay,
    revealPath,
    openPath,
    getAppPaths,
  } from "../lib/api";
  import type { AppSettings, UpdatePreferences, LauncherOverrides, AdvancedConfig, NetworkConfig, SgdbConfig, AppPathsDto } from "../lib/api";
  import { LAUNCHER_BRANDS, LAUNCHER_BRAND_ORDER, type LauncherBrandKey } from "../lib/launcherLogos";
  import { resetNudgeSession } from "../lib/community";
  import BrandMark from "../components/BrandMark.svelte";
  import Select from "../components/Select.svelte";
  import { t, locale, setLocale, translate, LOCALES, LOCALE_LABELS, type Locale } from "../lib/i18n/index";
  import { get } from "svelte/store";

  let { onToggleTheme, currentTheme }: { onToggleTheme: () => void; currentTheme: string } = $props();
  let dlssOverlayLive = $state(false);
  let appVersion = $state("dev");
  let appPaths = $state<AppPathsDto | null>(null);
  let updateChecking = $state(false);
  let lastUpdateCheck = $state<string | null>(null);

  type TabId = "general" | "updates" | "detection" | "art" | "advanced";

  const TAB_IDS: readonly TabId[] = ["general", "updates", "detection", "art", "advanced"];
  function tabFromPref(value: string | undefined | null): TabId {
    return (TAB_IDS as readonly string[]).includes(value ?? "") ? (value as TabId) : "general";
  }

  let activeTab = $state<TabId>(tabFromPref($settings?.ui_prefs.settings_active_tab));

  $effect(() => {
    const persisted = tabFromPref($settings?.ui_prefs.settings_active_tab);
    if (persisted !== activeTab) activeTab = persisted;
  });

  async function setActiveTab(id: TabId): Promise<void> {
    activeTab = id;
    if (!$settings || $settings.ui_prefs.settings_active_tab === id) return;
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, settings_active_tab: id } });
  }

  async function setShowSupportNudge(on: boolean): Promise<void> {
    if (!$settings) return;
    if (on) resetNudgeSession();
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, show_support_nudge: on } });
  }

  const TABS: { id: TabId; labelKey: string; icon: string }[] = [
    { id: "general", labelKey: "view.settings.tab.general", icon: "settings" },
    { id: "updates", labelKey: "view.settings.tab.updates", icon: "sync" },
    { id: "detection", labelKey: "view.settings.tab.detection", icon: "scan" },
    { id: "art", labelKey: "view.settings.tab.art", icon: "image" },
    { id: "advanced", labelKey: "view.settings.tab.advanced", icon: "flask" },
  ];

  const localeOptions = LOCALES.map((loc) => ({ value: loc, label: LOCALE_LABELS[loc] }));
  let localeChoice = $state<Locale>(get(locale));

  $effect(() => {
    localeChoice = $locale;
  });

  $effect(() => {
    if (localeChoice === get(locale)) return;
    const next = localeChoice;
    setLocale(next);
    if ($settings) {
      void persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, language: next } });
    }
  });

  onMount(async () => {
    await loadSettings();
    try {
      dlssOverlayLive = await getDlssDebugOverlay();
    } catch {
      dlssOverlayLive = false;
    }
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      appVersion = await getVersion();
    } catch {
      appVersion = "dev";
    }
    try {
      appPaths = await getAppPaths();
    } catch {
      appPaths = null;
    }
    if ($driverReports.length === 0) void loadDriverUpdates();
  });

  async function checkForUpdatesNow(): Promise<void> {
    if (updateChecking) return;
    updateChecking = true;
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      lastUpdateCheck = new Date().toLocaleString();
      const available = update && (update as { available?: boolean }).available !== false;
      if (available) {
        window.dispatchEvent(new CustomEvent("dlssync:check-updates", { detail: { force: true } }));
        const v = (update as { version?: string }).version ?? "unknown";
        showToast("info", translate(get(locale), "view.settings.updates.toast.available", { version: v }));
      } else {
        showToast("success", translate(get(locale), "view.settings.updates.toast.latest", { version: appVersion }));
      }
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "view.settings.updates.toast.checkFailed", { error: String(err) }));
    } finally {
      updateChecking = false;
    }
  }

  async function toggleOverlay(): Promise<void> {
    if (!$settings) return;
    const next = !dlssOverlayLive;
    try {
      await setDlssDebugOverlay(next);
      dlssOverlayLive = next;
      await persistSettings({
        ...$settings,
        advanced: { ...$settings.advanced, dlss_debug_overlay: next },
      });
      showToast("success", translate(get(locale), next ? "view.settings.toast.overlayEnabled" : "view.settings.toast.overlayDisabled"));
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "view.settings.toast.registryWrite", { error: String(err) }));
    }
  }

  function updateAdvanced<K extends keyof AdvancedConfig>(key: K, value: AdvancedConfig[K]): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      advanced: { ...$settings.advanced, [key]: value },
    });
  }

  function updateNetwork<K extends keyof NetworkConfig>(key: K, value: NetworkConfig[K]): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      network: { ...$settings.network, [key]: value },
    });
  }

  let customFolderInput = $state("");

  function updatePref<K extends keyof UpdatePreferences>(key: K, value: UpdatePreferences[K]): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      update_prefs: { ...$settings.update_prefs, [key]: value },
    });
  }

  function updateOverride<K extends keyof LauncherOverrides>(key: K, value: LauncherOverrides[K]): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      launcher_overrides: { ...$settings.launcher_overrides, [key]: value },
    });
  }

  async function addCustomFolder(): Promise<void> {
    if (!$settings) return;
    const raw = customFolderInput.trim();
    if (!raw) return;
    if ($settings.launcher_overrides.custom.includes(raw)) {
      showToast("warning", translate(get(locale), "view.settings.toast.folderAlreadyAdded"));
      return;
    }
    await persistSettings({
      ...$settings,
      launcher_overrides: {
        ...$settings.launcher_overrides,
        custom: [...$settings.launcher_overrides.custom, raw],
      },
    });
    customFolderInput = "";
    showToast("success", translate(get(locale), "view.settings.toast.folderAdded"));
  }

  async function pickCustomFolder(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({ directory: true, multiple: false });
      if (typeof result === "string" && result) {
        customFolderInput = result;
        await addCustomFolder();
      }
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "view.settings.toast.folderPickerFailed", { error: String(err) }));
    }
  }

  async function removeCustomFolder(path: string): Promise<void> {
    if (!$settings) return;
    await persistSettings({
      ...$settings,
      launcher_overrides: {
        ...$settings.launcher_overrides,
        custom: $settings.launcher_overrides.custom.filter((p) => p !== path),
      },
    });
  }

  const LAUNCHER_KEYS = LAUNCHER_BRAND_ORDER;
  type LauncherKey = LauncherBrandKey;

  function launcherLabel(key: LauncherKey): string {
    return LAUNCHER_BRANDS[key].label;
  }

  function updateSteamApi<K extends keyof AppSettings["steam_api"]>(
    key: K,
    value: AppSettings["steam_api"][K],
  ): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      steam_api: { ...$settings.steam_api, [key]: value },
    });
  }

  function updateSgdb<K extends keyof SgdbConfig>(key: K, value: SgdbConfig[K]): void {
    if (!$settings) return;
    void persistSettings({
      ...$settings,
      steamgriddb: { ...$settings.steamgriddb, [key]: value },
    });
  }

  async function openSgdbPrefs(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open("https://www.steamgriddb.com/profile/preferences/api");
    } catch {
      window.open("https://www.steamgriddb.com/profile/preferences/api", "_blank");
    }
  }

  async function openSteamKey(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open("https://steamcommunity.com/dev/apikey");
    } catch {
      window.open("https://steamcommunity.com/dev/apikey", "_blank");
    }
  }

  async function revealConfigFile(): Promise<void> {
    if (!appPaths) {
      showToast("warning", translate(get(locale), "view.settings.toast.pathsUnavailable"));
      return;
    }
    try {
      await revealPath(appPaths.settings_file);
    } catch (err: unknown) {
      try {
        await openPath(appPaths.settings_dir);
      } catch (err2: unknown) {
        showToast("danger", translate(get(locale), "view.settings.toast.revealFailed", { error: String(err2) }));
      }
    }
  }

  async function openConfigDir(): Promise<void> {
    if (!appPaths) return;
    try {
      await openPath(appPaths.root);
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "view.settings.toast.openFailed", { error: String(err) }));
    }
  }

  async function openBackupsDir(): Promise<void> {
    if (!appPaths) return;
    try {
      await openPath(appPaths.backups_dir);
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "view.settings.toast.openFailed", { error: String(err) }));
    }
  }

  async function openLogsDir(): Promise<void> {
    if (!appPaths) return;
    try {
      await openPath(appPaths.logs_dir);
    } catch (err: unknown) {
      try {
        await openPath(appPaths.root);
      } catch {
        showToast("danger", translate(get(locale), "view.settings.toast.openFailed", { error: String(err) }));
      }
    }
  }

  type FeatureToggle = { key: keyof UpdatePreferences; labelKey: string; subKey: string; files: string | null };
  const featureToggles: FeatureToggle[] = [
    { key: "update_dlss", labelKey: "view.settings.feature.update_dlss.label", subKey: "view.settings.feature.update_dlss.sub", files: "nvngx_dlss.dll · sl.dlss.dll" },
    { key: "update_dlss_fg", labelKey: "view.settings.feature.update_dlss_fg.label", subKey: "view.settings.feature.update_dlss_fg.sub", files: "nvngx_dlssg.dll · sl.dlss_g.dll" },
    { key: "update_dlss_rr", labelKey: "view.settings.feature.update_dlss_rr.label", subKey: "view.settings.feature.update_dlss_rr.sub", files: "nvngx_dlssd.dll · sl.dlss_d.dll" },
    { key: "update_streamline", labelKey: "view.settings.feature.update_streamline.label", subKey: "view.settings.feature.update_streamline.sub", files: "sl.interposer.dll · sl.common.dll · sl.dlss.dll · sl.dlss_g.dll · sl.reflex.dll · sl.pcl.dll · sl.directsr.dll" },
    { key: "update_reflex", labelKey: "view.settings.feature.update_reflex.label", subKey: "view.settings.feature.update_reflex.sub", files: "sl.reflex.dll" },
    { key: "update_xess", labelKey: "view.settings.feature.update_xess.label", subKey: "view.settings.feature.update_xess.sub", files: "libxess.dll · libxess_fg.dll · libxell.dll" },
    { key: "update_fsr", labelKey: "view.settings.feature.update_fsr.label", subKey: "view.settings.feature.update_fsr.sub", files: "amd_fidelityfx_*.dll · ffx_*.dll" },
    { key: "update_direct_storage", labelKey: "view.settings.feature.update_direct_storage.label", subKey: "view.settings.feature.update_direct_storage.sub", files: "dstorage.dll · dstoragecore.dll" },
  ];

  const updateBehaviorToggles: FeatureToggle[] = [
    { key: "create_backups", labelKey: "view.settings.feature.create_backups.label", subKey: "view.settings.feature.create_backups.sub", files: null },
    { key: "auto_apply_all_on_rescan", labelKey: "view.settings.feature.auto_apply_all_on_rescan.label", subKey: "view.settings.feature.auto_apply_all_on_rescan.sub", files: null },
  ];

  let showFilesFor = $state<Record<string, boolean>>({});
  function toggleFiles(key: string): void {
    showFilesFor = { ...showFilesFor, [key]: !showFilesFor[key] };
  }

  let sgdbKeyMasked = $derived.by(() => {
    const k = $settings?.steamgriddb.api_key ?? "";
    if (!k) return "";
    return k.length > 8 ? `${k.slice(0, 4)}…${k.slice(-4)}` : "•••";
  });

  let enabledFeatureCount = $derived.by(() => {
    if (!$settings) return 0;
    return featureToggles.filter((ft) => $settings!.update_prefs[ft.key]).length;
  });
</script>

<header class="view-header">
  <div>
    <h1 class="view-title">{$t("view.settings.title")}</h1>
    <p class="view-subtitle">{$t("view.settings.savedTo")} <span class="mono">{appPaths ? appPaths.settings_file : "~/DLSSync/Settings/settings.json"}</span></p>
  </div>
</header>

{#if !$settings}
  <div class="loading">{$t("view.settings.loading")}</div>
{:else}
  <section class="settings-hero" in:fly={{ y: 6, duration: 220 }}>
    <div class="hero-meta">
      <span class="hero-eyebrow">DLSSync</span>
      <div class="hero-title-row">
        <span class="hero-version mono">v{appVersion}</span>
        <span class="hero-status chip chip-update is-strong">{$t("view.settings.hero.technologiesEnabled", { count: enabledFeatureCount, total: featureToggles.length })}</span>
      </div>
      <p class="hero-sub">{$t("view.settings.hero.sub")}</p>
    </div>
    <div class="hero-actions">
      <button class="btn btn-sm btn-ghost" onclick={revealConfigFile} disabled={!appPaths} title={$t("view.settings.hero.revealConfigTitle")}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
        {$t("view.settings.hero.revealConfig")}
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openConfigDir} disabled={!appPaths} title={$t("view.settings.hero.dataFolderTitle")}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
        {$t("view.settings.hero.dataFolder")}
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openBackupsDir} disabled={!appPaths} title={$t("view.settings.hero.backupsTitle")}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
        {$t("view.settings.hero.backups")}
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openLogsDir} disabled={!appPaths} title={$t("view.settings.hero.logsTitle")}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="13 2 13 9 20 9"/><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><line x1="9" y1="13" x2="15" y2="13"/><line x1="9" y1="17" x2="15" y2="17"/></svg>
        {$t("view.settings.hero.logs")}
      </button>
    </div>
  </section>

  <div class="settings-layout">
    <nav class="side-nav" aria-label={$t("view.settings.sectionsAria")}>
      {#each TABS as tab}
        <button class="side-tab" class:active={activeTab === tab.id} onclick={() => void setActiveTab(tab.id)}>
          <span class="side-tab-icon" aria-hidden="true">
            {#if tab.icon === "settings"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
            {:else if tab.icon === "sync"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
            {:else if tab.icon === "scan"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7V4a1 1 0 0 1 1-1h3M17 3h3a1 1 0 0 1 1 1v3M21 17v3a1 1 0 0 1-1 1h-3M7 21H4a1 1 0 0 1-1-1v-3"/><circle cx="12" cy="12" r="3"/></svg>
            {:else if tab.icon === "image"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
            {:else if tab.icon === "flask"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 2v6L4 20a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2L15 8V2"/><line x1="8" y1="2" x2="16" y2="2"/></svg>
            {/if}
          </span>
          <span class="side-tab-label">{$t(tab.labelKey)}</span>
          <span class="side-tab-rail" aria-hidden="true"></span>
        </button>
      {/each}
    </nav>

  <div class="tab-panels">
    {#if activeTab === "general"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">{$t("view.settings.general.appearance.title")}</h2>
          <p class="section-help">{$t("view.settings.general.appearance.help")}</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.general.darkTheme.label")}</div>
              <div class="row-sub">{$t("view.settings.general.darkTheme.sub")}</div>
            </div>
            <label class="toggle">
              <input type="checkbox" checked={currentTheme === "dark"} onchange={onToggleTheme} />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("language.label")}</div>
              <div class="row-sub">{$t("view.settings.general.language.sub")}</div>
            </div>
            <div class="lang-select">
              <Select
                bind:value={localeChoice}
                options={localeOptions}
                ariaLabel={$t("language.switcherAria")}
              />
            </div>
          </div>
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">{$t("view.settings.general.performance.title")}</h2>
          <p class="section-help">{$t("view.settings.general.performance.help")}</p>
        </header>
        <PerformanceToggles />

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">{$t("view.settings.general.updateBehavior.title")}</h2>
          <p class="section-help">{$t("view.settings.general.updateBehavior.help")}</p>
        </header>
        <div class="card">
          {#each updateBehaviorToggles as ft, i}
            <div class="row" class:row-divider={i > 0}>
              <div class="row-text">
                <div class="row-label">{$t(ft.labelKey)}</div>
                <div class="row-sub">{$t(ft.subKey)}</div>
              </div>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={$settings.update_prefs[ft.key]}
                  onchange={(e) => updatePref(ft.key, (e.target as HTMLInputElement).checked)}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          {/each}
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">{$t("view.settings.general.support.title")}</h2>
          <p class="section-help">{$t("view.settings.general.support.help")}</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.general.support.label")}</div>
              <div class="row-sub">{$t("view.settings.general.support.sub")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={$settings.ui_prefs.show_support_nudge}
                onchange={(e) => void setShowSupportNudge((e.target as HTMLInputElement).checked)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>
      </section>

    {:else if activeTab === "updates"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">{$t("view.settings.updates.autoUpdate.title")}</h2>
          <p class="section-help">{$t("view.settings.updates.autoUpdate.help")}</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.updates.currentVersion.label")}</div>
              <div class="row-sub mono">v{appVersion}{lastUpdateCheck ? `  ·  ${$t("view.settings.updates.lastCheck", { time: lastUpdateCheck })}` : ""}</div>
            </div>
            <button class="btn btn-primary" onclick={checkForUpdatesNow} disabled={updateChecking}>
              {updateChecking ? $t("view.settings.updates.checking") : $t("view.settings.updates.checkNow")}
            </button>
          </div>
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">{$t("view.settings.updates.prefs.title")}</h2>
          <p class="section-help">{$t("view.settings.updates.prefs.help")}</p>
        </header>
        <div class="card">
          {#each featureToggles as ft, i}
            <div class="row" class:row-divider={i > 0}>
              <div class="row-text">
                <div class="row-label">{$t(ft.labelKey)}</div>
                <div class="row-sub">{$t(ft.subKey)}</div>
                {#if ft.files}
                  <button class="files-disclosure" onclick={() => toggleFiles(ft.key)} aria-expanded={!!showFilesFor[ft.key]}>
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={!!showFilesFor[ft.key]}><polyline points="6 9 12 15 18 9"/></svg>
                    {showFilesFor[ft.key] ? $t("view.settings.filesDisclosure.hide") : $t("view.settings.filesDisclosure.show")}
                  </button>
                  {#if showFilesFor[ft.key]}
                    <div class="files-meta mono">{ft.files}</div>
                  {/if}
                {/if}
              </div>
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={$settings.update_prefs[ft.key]}
                  onchange={(e) => updatePref(ft.key, (e.target as HTMLInputElement).checked)}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          {/each}
        </div>
      </section>

    {:else if activeTab === "detection"}
      {#snippet launcherLogo(key: LauncherKey)}
        <span class="launcher-logo" style:background={LAUNCHER_BRANDS[key].bg} aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="#ffffff" xmlns="http://www.w3.org/2000/svg">
            <path d={LAUNCHER_BRANDS[key].path} />
          </svg>
        </span>
      {/snippet}

      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">{$t("view.settings.detection.customFolders.title")}</h2>
          <p class="section-help">{$t("view.settings.detection.customFolders.help", { path: "C:\\Games" })}</p>
        </header>
        <div class="card">
          <div class="folder-input-row">
            <div class="folder-input-wrap">
              <svg class="folder-input-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
              <input
                type="text"
                placeholder="C:\Games"
                bind:value={customFolderInput}
                onkeydown={(e) => { if (e.key === "Enter") void addCustomFolder(); }}
              />
            </div>
            <button class="aura-pill aura-pill-ghost" onclick={pickCustomFolder}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
              {$t("view.settings.detection.browse")}
            </button>
            <button class="aura-pill aura-pill-primary" onclick={addCustomFolder} disabled={!customFolderInput.trim()}>
              {$t("view.settings.detection.addFolder")}
            </button>
          </div>
          {#if $settings.launcher_overrides.custom.length === 0}
            <div class="empty-state">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" opacity="0.4"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
              <span>{$t("view.settings.detection.noFolders")}</span>
            </div>
          {:else}
            <ul class="path-list">
              {#each $settings.launcher_overrides.custom as p (p)}
                <li class="path-row">
                  <span class="path-icon aura-badge" data-tint="blue">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                  </span>
                  <span class="path-text mono">{p}</span>
                  <button class="path-action" title={$t("view.settings.detection.openFolderTitle")} onclick={() => openPath(p).catch((err) => showToast("danger", translate(get(locale), "view.settings.toast.openFailed", { error: String(err) })))}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                  </button>
                  <button class="path-action path-action-danger" title={$t("view.settings.detection.removeTitle")} onclick={() => removeCustomFolder(p)}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="row-actions">
            <button class="aura-pill aura-pill-ghost" onclick={() => scanGames()}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
              {$t("view.settings.detection.rescanNow")}
            </button>
          </div>
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">{$t("view.settings.detection.launcherOverrides.title")}</h2>
          <p class="section-help">{$t("view.settings.detection.launcherOverrides.help")}</p>
        </header>
        <div class="card launcher-card">
          {#each LAUNCHER_KEYS as launcher, i}
            {@const arr = ($settings.launcher_overrides as unknown as Record<string, string[]>)[launcher] ?? []}
            <div class="launcher-row" class:row-divider={i > 0}>
              <div class="launcher-head">
                {@render launcherLogo(launcher)}
                <div class="launcher-head-text">
                  <div class="row-label">{launcherLabel(launcher)}</div>
                  <div class="row-sub">{arr.length === 0 ? $t("view.settings.detection.launcher.defaultAuto") : $t("view.settings.detection.launcher.overrideCount", { count: arr.length })}</div>
                </div>
              </div>
              <div class="launcher-input-col">
                {#each arr as p, idx (idx)}
                  <div class="path-input-row">
                    <input
                      type="text"
                      value={p}
                      placeholder="C:\Program Files\..."
                      onchange={(e) => {
                        const next = [...arr];
                        next[idx] = (e.target as HTMLInputElement).value;
                        updateOverride(launcher as keyof LauncherOverrides, next);
                      }}
                    />
                    <button class="path-remove" title={$t("view.settings.detection.launcher.removePathTitle")} onclick={() => updateOverride(launcher as keyof LauncherOverrides, arr.filter((_, j) => j !== idx))}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="6" x2="18" y2="18"/><line x1="6" y1="18" x2="18" y2="6"/></svg>
                    </button>
                  </div>
                {/each}
                <button class="add-path-pill" onclick={() => updateOverride(launcher as keyof LauncherOverrides, [...arr, ""])}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                  {$t("view.settings.detection.launcher.addPath")}
                </button>
              </div>
            </div>
          {/each}
        </div>
      </section>

    {:else if activeTab === "art"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">{$t("view.settings.art.steamCdn.title")}</h2>
          <p class="section-help">{$t("view.settings.art.steamCdn.help")}</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.art.steamCdn.alwaysOn")} <span class="chip chip-success small-pill">{$t("view.settings.art.steamCdn.noKeyRequired")}</span></div>
              <div class="row-sub">{$t("view.settings.art.steamCdn.sub")}</div>
            </div>
          </div>
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">SteamGridDB <span class="section-tag">{$t("view.settings.art.sgdb.fallback")}</span></h2>
          <p class="section-help">{$t("view.settings.art.sgdb.help")}</p>
          <p class="section-help muted">{$t("view.settings.art.sgdb.helpMuted")}</p>
        </header>
        <div class="card">
          <div class="row art-row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.art.sgdb.label")} {#if sgdbKeyMasked}<span class="chip chip-update small-pill">{$t("view.settings.art.sgdb.active", { masked: sgdbKeyMasked })}</span>{/if}</div>
              <div class="row-sub">{$t("view.settings.art.sgdb.sub", { file: "settings.json" })}</div>
              <button class="files-disclosure" onclick={openSgdbPrefs}>
                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
                {$t("view.settings.art.sgdb.getKey")}
              </button>
            </div>
            <input
              type="password"
              placeholder={$t("view.settings.art.sgdb.placeholder")}
              value={$settings.steamgriddb.api_key}
              onchange={(e) => updateSgdb("api_key", (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>

        <header class="section-head section-head-gap">
          <h2 class="section-title-h">Steam Web API <span class="section-tag">{$t("view.settings.art.steamApi.optional")}</span></h2>
          <p class="section-help">{$t("view.settings.art.steamApi.help", { manifest: "appmanifest_*.acf", site: "steamcommunity.com/dev/apikey" })}</p>
          <p class="section-help muted">{$t("view.settings.art.steamApi.helpMuted", { file: "settings.json" })}</p>
        </header>
        <div class="card">
          <div class="row art-row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.art.steamApi.keyLabel")}</div>
              <div class="row-sub">{$t("view.settings.art.steamApi.keySub")}</div>
              <button class="files-disclosure" onclick={openSteamKey}>
                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
                steamcommunity.com/dev/apikey
              </button>
            </div>
            <input
              type="password"
              placeholder={$t("view.settings.art.steamApi.keyPlaceholder")}
              value={$settings.steam_api.api_key}
              onchange={(e) => updateSteamApi("api_key", (e.target as HTMLInputElement).value)}
            />
          </div>
          <div class="row art-row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.art.steamApi.idLabel")}</div>
              <div class="row-sub">{$t("view.settings.art.steamApi.idSub", { file: "loginusers.vdf" })}</div>
            </div>
            <input
              type="text"
              placeholder="76561198xxxxxxxxx"
              value={$settings.steam_api.steam_id}
              onchange={(e) => updateSteamApi("steam_id", (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>
      </section>

    {:else if activeTab === "advanced"}
      <section in:fly={{ y: 4, duration: 200 }} class="tab-section">
        <header class="section-head">
          <h2 class="section-title-h">{$t("view.settings.advanced.powerUser.title")}</h2>
          <p class="section-help">{$t("view.settings.advanced.powerUser.help", { regPath: "HKCU\\SOFTWARE\\NVIDIA Corporation\\Global\\NGXCore" })}</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.overlay.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.overlay.sub")}</div>
            </div>
            <label class="toggle">
              <input type="checkbox" checked={dlssOverlayLive} onchange={toggleOverlay} />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.verboseLogs.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.verboseLogs.sub", { dir: "logs/" })}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={$settings.advanced.verbose_logs}
                onchange={(e) => updateAdvanced("verbose_logs", (e.target as HTMLInputElement).checked)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.preferStable.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.preferStable.sub")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={$settings.advanced.prefer_stable_channel}
                onchange={(e) => updateAdvanced("prefer_stable_channel", (e.target as HTMLInputElement).checked)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">
                {$t("view.settings.advanced.allowUnsigned.label")}
                <span class="chip chip-warning small-pill">{$t("view.settings.advanced.allowUnsigned.devOnly")}</span>
              </div>
              <div class="row-sub">{$t("view.settings.advanced.allowUnsigned.sub")}</div>
            </div>
            <label class="toggle">
              <input
                type="checkbox"
                checked={$settings.advanced.allow_unsigned_dlls}
                onchange={(e) => updateAdvanced("allow_unsigned_dlls", (e.target as HTMLInputElement).checked)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.parallelApplies.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.parallelApplies.sub")}</div>
            </div>
            <input
              type="number"
              class="inline-num"
              min="1"
              max="4"
              step="1"
              value={$settings.advanced.apply_concurrency}
              onchange={(e) => updateAdvanced("apply_concurrency", Math.max(1, Math.min(4, Number((e.target as HTMLInputElement).value) || 2)))}
            />
          </div>
        </div>

        <div class="settings-card">
          <header class="card-head">
            <h3>{$t("view.settings.advanced.network.title")}</h3>
            <p class="card-sub">{$t("view.settings.advanced.network.sub")}</p>
          </header>
          <div class="row">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.network.retry.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.network.retry.sub")}</div>
            </div>
            <input
              type="number"
              class="inline-num"
              min="1"
              max="6"
              step="1"
              value={$settings.network.retry_attempts}
              onchange={(e) => updateNetwork("retry_attempts", Math.max(1, Math.min(6, Number((e.target as HTMLInputElement).value) || 3)))}
            />
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.network.chunkTimeout.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.network.chunkTimeout.sub")}</div>
            </div>
            <input
              type="number"
              class="inline-num"
              min="10"
              max="600"
              step="10"
              value={$settings.network.chunk_timeout_secs}
              onchange={(e) => updateNetwork("chunk_timeout_secs", Math.max(10, Math.min(600, Number((e.target as HTMLInputElement).value) || 60)))}
            />
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.network.connectTimeout.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.network.connectTimeout.sub")}</div>
            </div>
            <input
              type="number"
              class="inline-num"
              min="3"
              max="60"
              step="1"
              value={$settings.network.connect_timeout_secs}
              onchange={(e) => updateNetwork("connect_timeout_secs", Math.max(3, Math.min(60, Number((e.target as HTMLInputElement).value) || 10)))}
            />
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">{$t("view.settings.advanced.network.cacheTtl.label")}</div>
              <div class="row-sub">{$t("view.settings.advanced.network.cacheTtl.sub")}</div>
            </div>
            <input
              type="number"
              class="inline-num"
              min="60"
              max="3600"
              step="60"
              value={$settings.network.download_cache_ttl_secs}
              onchange={(e) => updateNetwork("download_cache_ttl_secs", Math.max(60, Math.min(3600, Number((e.target as HTMLInputElement).value) || 300)))}
            />
          </div>
        </div>

        <div class="settings-card">
          <header class="card-head">
            <h3>{$t("view.settings.advanced.dlssOverrides.title")} <span class="chip chip-update small-pill"><BrandMark key="nvidia" tone="mono" size={11} /></span></h3>
            <p class="card-sub">{$t("view.settings.advanced.dlssOverrides.sub", { drivers: $t("view.settings.advanced.dlssOverrides.driversTabBold") })}</p>
          </header>
          <button class="btn btn-primary card-cta" onclick={() => currentView.set("drivers")}>{$t("view.settings.advanced.dlssOverrides.openDrivers")}</button>
        </div>
      </section>
    {/if}
  </div>
  </div>
{/if}

<style>
  .view-header { margin-bottom: var(--space-5); }
  .loading { padding: 60px 0; text-align: center; color: var(--text-muted); }

  .settings-hero {
    margin-bottom: var(--space-6);
    padding: var(--space-5) var(--space-6);
    border-radius: var(--radius-xl);
    background: linear-gradient(135deg, var(--bg-card), var(--bg-elevated));
    border: 1px solid var(--border);
    box-shadow: var(--shadow-xs);
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-5);
    align-items: center;
  }
  @media (max-width: 640px) {
    .settings-hero { grid-template-columns: 1fr; }
  }
  .hero-eyebrow {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--accent);
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
  }
  .hero-title-row { display: flex; align-items: center; gap: var(--space-3); margin-top: var(--space-1); flex-wrap: wrap; }
  .hero-version { font-size: var(--fs-xl); font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tighter); font-variant-numeric: tabular-nums; }
  .hero-status { padding: 4px 10px; font-size: var(--fs-2xs); }
  .hero-sub { font-size: var(--fs-sm); color: var(--text-secondary); margin-top: var(--space-2); line-height: var(--lh-snug); max-width: 540px; }
  .hero-actions { display: flex; gap: var(--space-2); flex-wrap: wrap; }

  .settings-layout {
    display: grid;
    grid-template-columns: 224px minmax(0, 1fr);
    gap: clamp(var(--space-5), 3vw, var(--space-8));
    align-items: start;
  }
  @media (max-width: 900px) {
    .settings-layout { grid-template-columns: 1fr; gap: var(--space-4); }
  }
  .side-nav {
    position: sticky;
    top: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    border-radius: var(--radius-xl);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }
  @media (max-width: 900px) {
    .side-nav { position: static; flex-direction: row; flex-wrap: wrap; gap: var(--space-1); }
    .side-tab { flex: 1 1 auto; }
  }
  .side-tab {
    position: relative;
    display: grid;
    grid-template-columns: 22px 1fr;
    align-items: center;
    gap: var(--space-3);
    min-height: 40px;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .side-tab:hover { background: var(--bg-elevated); color: var(--text-primary); }
  .side-tab:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .side-tab-icon { color: var(--text-muted); display: inline-flex; transition: color var(--dur-fast) var(--ease); }
  .side-tab.active { background: var(--accent-dim); color: var(--accent); }
  .side-tab.active .side-tab-icon { color: var(--accent); }
  .side-tab-rail {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 56%;
    border-radius: var(--radius-full);
    background: transparent;
    transition: background var(--dur-fast) var(--ease);
  }
  .side-tab.active .side-tab-rail { background: var(--accent); }

  .tab-panels { max-width: 960px; min-width: 0; container-type: inline-size; }
  .tab-panels > :global(section) {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .section-head {
    display: block;
    margin: 0;
    padding-left: var(--space-3);
    border-left: 2px solid var(--accent);
  }
  .section-head-gap { margin-top: var(--space-5); }
  .section-title-h {
    font-size: var(--fs-md);
    font-weight: 700;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
    margin-bottom: 2px;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    text-transform: none;
  }
  .section-tag {
    font-size: var(--fs-2xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    padding: 1px 7px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    font-weight: 600;
  }
  .section-help {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    line-height: var(--lh-snug);
    max-width: 64ch;
  }
  .section-help.muted { font-size: var(--fs-xs); color: var(--text-muted); margin-top: var(--space-1); }

  .card { padding: var(--space-1) var(--space-5); }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-4) 0;
  }
  .art-row {
    align-items: start;
    grid-template-columns: minmax(0, 1fr) clamp(200px, 38%, 300px);
    gap: var(--space-4);
  }
  @container (max-width: 520px) {
    .art-row { grid-template-columns: 1fr; gap: var(--space-2); }
  }
  .row-divider { border-top: 1px solid var(--border); }
  .inline-num {
    width: 88px;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    text-align: right;
    flex-shrink: 0;
    transition: border-color var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }
  .inline-num:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .lang-select { width: 200px; max-width: 100%; flex-shrink: 0; }
  .art-row input[type="text"],
  .art-row input[type="password"] {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    transition: border-color var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }
  .art-row input::placeholder { color: var(--text-placeholder); }
  .art-row input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .row-label { font-size: var(--fs-base); font-weight: 600; color: var(--text-primary); display: inline-flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
  .row-sub { font-size: var(--fs-xs); color: var(--text-secondary); margin-top: 3px; line-height: var(--lh-normal); }
  .row-text { min-width: 0; }
  .settings-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-1) var(--space-5) var(--space-5);
    min-width: 0;
  }
  .card-head {
    padding: var(--space-4) 0 var(--space-2);
    margin-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
  }
  .card-head h3 {
    font-size: var(--fs-base);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .card-sub { font-size: var(--fs-xs); color: var(--text-secondary); margin-top: 3px; line-height: var(--lh-snug); }
  .card-cta { margin-top: var(--space-4); }

  .files-disclosure {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: var(--space-2);
    font-size: var(--fs-2xs);
    font-weight: 600;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 0;
    border-radius: var(--radius-xs);
    transition: color var(--dur-fast) var(--ease);
  }
  .files-disclosure:hover { color: var(--accent); }
  .files-disclosure:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .files-disclosure .chev { transition: transform var(--dur-fast) var(--ease); }
  .files-disclosure .chev.open { transform: rotate(180deg); }
  .files-meta { margin-top: var(--space-2); padding: var(--space-2) var(--space-3); background: var(--bg-input); border: 1px solid var(--border); border-radius: var(--radius-sm); font-size: var(--fs-2xs); color: var(--text-secondary); }

  .folder-input-row { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-4) 0 var(--space-1); flex-wrap: wrap; }
  .folder-input-wrap {
    flex: 1 1 240px;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: 42px;
    padding: 0 var(--space-4);
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    transition: border-color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }
  .folder-input-wrap:focus-within {
    border-color: var(--accent);
    background: var(--bg-card);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
  .folder-input-icon { color: var(--text-muted); flex-shrink: 0; }
  .folder-input-wrap input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    padding: 0;
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    color: var(--text-primary);
  }
  .folder-input-wrap input:focus { outline: none; }
  .folder-input-wrap input::placeholder { color: var(--text-placeholder); }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-6) 0 var(--space-5);
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .path-list { list-style: none; padding: var(--space-3) 0 var(--space-1); margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
  .path-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .path-row:hover { background: var(--bg-card-hover); border-color: var(--border-hover); }
  .path-icon { width: 32px; height: 32px; border-radius: var(--radius-md); }
  .path-text { font-size: var(--fs-xs); color: var(--text-primary); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path-action {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
  }
  .path-action:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .path-action:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .path-action.path-action-danger:hover { background: var(--badge-red-bg); color: var(--badge-red-fg); }
  .row-actions { padding: var(--space-3) 0 var(--space-4); }

  .launcher-card { padding: var(--space-2) var(--space-5); }
  .launcher-row {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    gap: var(--space-5);
    align-items: start;
    padding: var(--space-4) 0;
  }
  @container (max-width: 560px) {
    .launcher-row { grid-template-columns: 1fr; gap: var(--space-3); }
  }
  .launcher-head { display: flex; align-items: center; gap: var(--space-3); min-width: 0; }
  .launcher-logo {
    width: 40px;
    height: 40px;
    border-radius: 11px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: var(--shadow-xs), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  }
  .launcher-logo svg { display: block; }
  .launcher-head-text { min-width: 0; }
  .launcher-input-col { display: flex; flex-direction: column; gap: var(--space-2); min-width: 0; }
  .path-input-row { display: flex; align-items: stretch; gap: var(--space-2); height: 38px; }
  .path-input-row input {
    flex: 1;
    min-width: 0;
    padding: 0 var(--space-3);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    color: var(--text-primary);
    transition: border-color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }
  .path-input-row input:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--bg-card);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .path-remove {
    width: 36px;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
  }
  .path-remove:hover { background: var(--badge-red-bg); color: var(--badge-red-fg); }
  .path-remove:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .add-path-pill {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 14px;
    border-radius: var(--radius-full);
    background: transparent;
    border: 1px dashed var(--border-strong);
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease), border-color 0.12s var(--ease);
  }
  .add-path-pill:hover {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent);
    border-style: solid;
  }
  .add-path-pill:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  @media (prefers-reduced-motion: reduce) {
    .side-tab, .side-tab-icon, .side-tab-rail, .inline-num, .files-disclosure,
    .files-disclosure .chev, .folder-input-wrap, .path-row, .path-action,
    .path-remove, .path-input-row input, .add-path-pill { transition: none; }
  }
</style>
