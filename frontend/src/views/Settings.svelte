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

  const TABS: { id: TabId; label: string; icon: string }[] = [
    { id: "general", label: "General", icon: "settings" },
    { id: "updates", label: "Update preferences", icon: "sync" },
    { id: "detection", label: "Detection", icon: "scan" },
    { id: "art", label: "Game art", icon: "image" },
    { id: "advanced", label: "Advanced", icon: "flask" },
  ];

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
        showToast("info", `v${v} is available — see the banner.`);
      } else {
        showToast("success", `You're on the latest version (v${appVersion}).`);
      }
    } catch (err: unknown) {
      showToast("danger", `Update check failed: ${String(err)}`);
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
      showToast("success", `DLSS Debug Overlay ${next ? "enabled" : "disabled"}`);
    } catch (err: unknown) {
      showToast("danger", `Registry write: ${String(err)}`);
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
      showToast("warning", "Folder already added");
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
    showToast("success", "Custom folder added — rescan to apply");
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
      showToast("danger", `Folder picker failed: ${String(err)}`);
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
      showToast("warning", "Data paths not available yet");
      return;
    }
    try {
      await revealPath(appPaths.settings_file);
    } catch (err: unknown) {
      try {
        await openPath(appPaths.settings_dir);
      } catch (err2: unknown) {
        showToast("danger", `Reveal failed: ${String(err2)}`);
      }
    }
  }

  async function openConfigDir(): Promise<void> {
    if (!appPaths) return;
    try {
      await openPath(appPaths.root);
    } catch (err: unknown) {
      showToast("danger", `Open failed: ${String(err)}`);
    }
  }

  async function openBackupsDir(): Promise<void> {
    if (!appPaths) return;
    try {
      await openPath(appPaths.backups_dir);
    } catch (err: unknown) {
      showToast("danger", `Open failed: ${String(err)}`);
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
        showToast("danger", `Open failed: ${String(err)}`);
      }
    }
  }

  type FeatureToggle = { key: keyof UpdatePreferences; label: string; sub: string; files: string | null };
  const featureToggles: FeatureToggle[] = [
    { key: "update_dlss", label: "DLSS Super Resolution", sub: "Sharper image at higher FPS — NVIDIA AI upscaling.", files: "nvngx_dlss.dll · sl.dlss.dll" },
    { key: "update_dlss_fg", label: "DLSS Frame Generation", sub: "Extra interpolated frames for smoother motion on RTX 40+ GPUs.", files: "nvngx_dlssg.dll · sl.dlss_g.dll" },
    { key: "update_dlss_rr", label: "DLSS Ray Reconstruction", sub: "Cleaner ray-traced reflections, shadows and global illumination.", files: "nvngx_dlssd.dll · sl.dlss_d.dll" },
    { key: "update_streamline", label: "NVIDIA Streamline runtime", sub: "Shared NVIDIA loader required by DLSS, Reflex and DirectSR.", files: "sl.interposer.dll · sl.common.dll · sl.pcl.dll · sl.nis.dll · sl.directsr.dll" },
    { key: "update_reflex", label: "NVIDIA Reflex", sub: "Lower input latency in supported titles.", files: "sl.reflex.dll" },
    { key: "update_xess", label: "Intel XeSS", sub: "Intel AI upscaling — best on Arc GPUs, works elsewhere.", files: "libxess.dll · libxess_fg.dll · libxell.dll" },
    { key: "update_fsr", label: "AMD FSR", sub: "AMD upscaling and frame generation — runs on any GPU.", files: "amd_fidelityfx_*.dll · ffx_*.dll" },
    { key: "update_direct_storage", label: "Microsoft DirectStorage", sub: "Faster game loading via direct NVMe → GPU streaming.", files: "dstorage.dll · dstoragecore.dll" },
  ];

  const updateBehaviorToggles: FeatureToggle[] = [
    { key: "create_backups", label: "Create backups before applying", sub: "Highly recommended. Required for one-click restore.", files: null },
    { key: "auto_apply_all_on_rescan", label: "Auto-apply updates on rescan", sub: "Off by default. Enable for unattended updates.", files: null },
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
    <h1 class="view-title">Settings</h1>
    <p class="view-subtitle">Saved to <span class="mono">{appPaths ? appPaths.settings_file : "~/DLSSync/Settings/settings.json"}</span></p>
  </div>
</header>

{#if !$settings}
  <div class="loading">Loading…</div>
{:else}
  <section class="settings-hero" in:fly={{ y: 6, duration: 220 }}>
    <div class="hero-meta">
      <span class="hero-eyebrow">DLSSync</span>
      <div class="hero-title-row">
        <span class="hero-version mono">v{appVersion}</span>
        <span class="hero-status chip chip-update is-strong">{enabledFeatureCount}/{featureToggles.length} technologies enabled</span>
      </div>
      <p class="hero-sub">All changes save instantly. The config file is plain JSON — feel free to inspect or back up.</p>
    </div>
    <div class="hero-actions">
      <button class="btn btn-sm btn-ghost" onclick={revealConfigFile} disabled={!appPaths} title="Reveal settings.json in Explorer">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
        Reveal config
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openConfigDir} disabled={!appPaths} title="Open data folder root (~/DLSSync)">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
        Data folder
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openBackupsDir} disabled={!appPaths} title="Open Backups folder">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
        Backups
      </button>
      <button class="btn btn-sm btn-ghost" onclick={openLogsDir} disabled={!appPaths} title="Open logs folder">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="13 2 13 9 20 9"/><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><line x1="9" y1="13" x2="15" y2="13"/><line x1="9" y1="17" x2="15" y2="17"/></svg>
        Logs
      </button>
    </div>
  </section>

  <div class="settings-layout">
    <nav class="side-nav" aria-label="Settings sections">
      {#each TABS as t}
        <button class="side-tab" class:active={activeTab === t.id} onclick={() => void setActiveTab(t.id)}>
          <span class="side-tab-icon" aria-hidden="true">
            {#if t.icon === "settings"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
            {:else if t.icon === "sync"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
            {:else if t.icon === "scan"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7V4a1 1 0 0 1 1-1h3M17 3h3a1 1 0 0 1 1 1v3M21 17v3a1 1 0 0 1-1 1h-3M7 21H4a1 1 0 0 1-1-1v-3"/><circle cx="12" cy="12" r="3"/></svg>
            {:else if t.icon === "image"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
            {:else if t.icon === "flask"}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 2v6L4 20a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2L15 8V2"/><line x1="8" y1="2" x2="16" y2="2"/></svg>
            {/if}
          </span>
          <span class="side-tab-label">{t.label}</span>
          <span class="side-tab-rail" aria-hidden="true"></span>
        </button>
      {/each}
    </nav>

  <div class="tab-panels">
    {#if activeTab === "general"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">Appearance</h2>
          <p class="section-help">Theme is persisted per machine; the picker is also live in the top bar.</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">Dark theme</div>
              <div class="row-sub">Pure-black background. OLED-friendly, lower glare at night.</div>
            </div>
            <label class="toggle">
              <input type="checkbox" checked={currentTheme === "dark"} onchange={onToggleTheme} />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">Performance &amp; startup</h2>
          <p class="section-help">Footprint controls. With all three enabled, DLSSync sits in the tray with near-zero CPU between scheduled update checks.</p>
        </header>
        <PerformanceToggles />

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">Update behavior</h2>
          <p class="section-help">Safety and automation toggles. Backups are recommended; auto-apply is opt-in.</p>
        </header>
        <div class="card">
          {#each updateBehaviorToggles as ft, i}
            <div class="row" class:row-divider={i > 0}>
              <div class="row-text">
                <div class="row-label">{ft.label}</div>
                <div class="row-sub">{ft.sub}</div>
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

    {:else if activeTab === "updates"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">Auto-update</h2>
          <p class="section-help">DLSSync polls GitHub Releases every 6 hours. Trigger a manual check below.</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">Current version</div>
              <div class="row-sub mono">v{appVersion}{lastUpdateCheck ? `  ·  last check: ${lastUpdateCheck}` : ""}</div>
            </div>
            <button class="btn btn-primary" onclick={checkForUpdatesNow} disabled={updateChecking}>
              {updateChecking ? "Checking…" : "Check for updates"}
            </button>
          </div>
        </div>

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">Update preferences</h2>
          <p class="section-help">Choose which technologies DLSSync will sync. Disabled families are still detected but never overwritten.</p>
        </header>
        <div class="card">
          {#each featureToggles as ft, i}
            <div class="row" class:row-divider={i > 0}>
              <div class="row-text">
                <div class="row-label">{ft.label}</div>
                <div class="row-sub">{ft.sub}</div>
                {#if ft.files}
                  <button class="files-disclosure" onclick={() => toggleFiles(ft.key)} aria-expanded={!!showFilesFor[ft.key]}>
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={!!showFilesFor[ft.key]}><polyline points="6 9 12 15 18 9"/></svg>
                    {showFilesFor[ft.key] ? "Hide files" : "Show technical files"}
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
          <h2 class="section-title-h">Custom folders</h2>
          <p class="section-help">Add directories where you keep games outside of standard launchers (e.g. <span class="mono">C:\Games</span>). Each subfolder will appear as a Manual entry in the Library.</p>
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
              Browse
            </button>
            <button class="aura-pill aura-pill-primary" onclick={addCustomFolder} disabled={!customFolderInput.trim()}>
              Add folder
            </button>
          </div>
          {#if $settings.launcher_overrides.custom.length === 0}
            <div class="empty-state">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" opacity="0.4"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
              <span>No custom folders yet</span>
            </div>
          {:else}
            <ul class="path-list">
              {#each $settings.launcher_overrides.custom as p (p)}
                <li class="path-row">
                  <span class="path-icon aura-badge" data-tint="blue">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                  </span>
                  <span class="path-text mono">{p}</span>
                  <button class="path-action" title="Open folder" onclick={() => openPath(p).catch((err) => showToast("danger", `Open failed: ${String(err)}`))}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                  </button>
                  <button class="path-action path-action-danger" title="Remove" onclick={() => removeCustomFolder(p)}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="row-actions">
            <button class="aura-pill aura-pill-ghost" onclick={() => scanGames()}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
              Rescan now
            </button>
          </div>
        </div>

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">Launcher overrides</h2>
          <p class="section-help">Default paths come from the Windows registry. Add a manual fallback only if a launcher is installed outside its standard location.</p>
        </header>
        <div class="card launcher-card">
          {#each LAUNCHER_KEYS as launcher, i}
            {@const arr = ($settings.launcher_overrides as unknown as Record<string, string[]>)[launcher] ?? []}
            <div class="launcher-row" class:row-divider={i > 0}>
              <div class="launcher-head">
                {@render launcherLogo(launcher)}
                <div class="launcher-head-text">
                  <div class="row-label">{launcherLabel(launcher)}</div>
                  <div class="row-sub">{arr.length === 0 ? "Default (auto)" : `${arr.length} override${arr.length > 1 ? "s" : ""}`}</div>
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
                    <button class="path-remove" title="Remove path" onclick={() => updateOverride(launcher as keyof LauncherOverrides, arr.filter((_, j) => j !== idx))}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="6" x2="18" y2="18"/><line x1="6" y1="18" x2="18" y2="6"/></svg>
                    </button>
                  </div>
                {/each}
                <button class="add-path-pill" onclick={() => updateOverride(launcher as keyof LauncherOverrides, [...arr, ""])}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                  Add path
                </button>
              </div>
            </div>
          {/each}
        </div>
      </section>

    {:else if activeTab === "art"}
      <section in:fly={{ y: 4, duration: 200 }}>
        <header class="section-head">
          <h2 class="section-title-h">Steam public CDN</h2>
          <p class="section-help">Default. DLSSync resolves your manual/custom games against the Steam store search and fetches the public CDN art (header, hero, capsule). No key required.</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">Always-on <span class="chip chip-success small-pill">no key required</span></div>
              <div class="row-sub">Every game without art is resolved automatically. Falls back to letter glyph for titles Steam doesn't index.</div>
            </div>
          </div>
        </div>

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">SteamGridDB <span class="section-tag">fallback</span></h2>
          <p class="section-help">For pirated, modded or super-niche titles that Steam doesn't index, SteamGridDB has community art. Free key on your SGDB profile.</p>
          <p class="section-help muted">We don't ship a bundled key — SteamGridDB rate-limits per key (1,000 req/day free tier), so a shared key would exhaust the quota across all DLSSync users.</p>
        </header>
        <div class="card">
          <div class="row art-row">
            <div class="row-text">
              <div class="row-label">SteamGridDB API key {#if sgdbKeyMasked}<span class="chip chip-update small-pill">Active · {sgdbKeyMasked}</span>{/if}</div>
              <div class="row-sub">Used only as fallback. Stored locally in <span class="mono">settings.json</span>.</div>
              <button class="files-disclosure" onclick={openSgdbPrefs}>
                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
                Get free key on steamgriddb.com
              </button>
            </div>
            <input
              type="password"
              placeholder="Leave empty to use Steam CDN only"
              value={$settings.steamgriddb.api_key}
              onchange={(e) => updateSgdb("api_key", (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>

        <header class="section-head" style="margin-top: 20px;">
          <h2 class="section-title-h">Steam Web API <span class="section-tag">optional</span></h2>
          <p class="section-help">Improves Steam title matching beyond what local <span class="mono">appmanifest_*.acf</span> already provides. Get a free key at <span class="mono">steamcommunity.com/dev/apikey</span>.</p>
          <p class="section-help muted">We don't ship a bundled key — Valve's Steam Web API Terms §2 require each key holder to keep it confidential and not share with third parties. Each user's key stays in their own <span class="mono">settings.json</span>.</p>
        </header>
        <div class="card">
          <div class="row art-row">
            <div class="row-text">
              <div class="row-label">API key</div>
              <div class="row-sub">32-character hex string.</div>
              <button class="files-disclosure" onclick={openSteamKey}>
                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
                steamcommunity.com/dev/apikey
              </button>
            </div>
            <input
              type="password"
              placeholder="Leave empty to use public CDN art"
              value={$settings.steam_api.api_key}
              onchange={(e) => updateSteamApi("api_key", (e.target as HTMLInputElement).value)}
            />
          </div>
          <div class="row art-row row-divider">
            <div class="row-text">
              <div class="row-label">Steam 64-bit ID</div>
              <div class="row-sub">Auto-detected from <span class="mono">loginusers.vdf</span> when blank.</div>
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
          <h2 class="section-title-h">Power-user toggles</h2>
          <p class="section-help">DLSS Debug Overlay writes to <span class="mono">HKCU\SOFTWARE\NVIDIA Corporation\Global\NGXCore</span>. Restart the game for changes to apply.</p>
        </header>
        <div class="card">
          <div class="row">
            <div class="row-text">
              <div class="row-label">DLSS Debug Overlay</div>
              <div class="row-sub">Adds NVIDIA's on-screen overlay showing DLSS version, mode, and frame timing.</div>
            </div>
            <label class="toggle">
              <input type="checkbox" checked={dlssOverlayLive} onchange={toggleOverlay} />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <div class="row row-divider">
            <div class="row-text">
              <div class="row-label">Verbose logs</div>
              <div class="row-sub">Capture full debug output to <span class="mono">logs/</span> under the data folder.</div>
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
              <div class="row-label">Prefer stable channel</div>
              <div class="row-sub">Hide experimental and beta builds from the version picker. Recommended.</div>
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
                Allow unsigned DLLs
                <span class="chip chip-warning small-pill">dev only</span>
              </div>
              <div class="row-sub">Bypass the Authenticode publisher gate. Every apply still checks SHA-256, but the vendor signature is no longer required. Off by default.</div>
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
              <div class="row-label">Parallel applies</div>
              <div class="row-sub">How many vendor-archive groups to install in parallel. Higher = faster on fat connections, lower = gentler on flaky links. 1–4, default 2.</div>
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

        <div class="settings-card" style="margin-top: 18px;">
          <header class="card-head">
            <h3>Network</h3>
            <p class="card-sub">Download client tunables — defaults are sane; tweak only if your link is unusual.</p>
          </header>
          <div class="row">
            <div class="row-text">
              <div class="row-label">Retry attempts per download</div>
              <div class="row-sub">How many times a flaky archive download retries before failing. Exponential backoff between attempts. 1–6, default 3.</div>
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
              <div class="row-label">Per-chunk timeout (seconds)</div>
              <div class="row-sub">Maximum gap allowed between byte chunks before aborting and retrying. 10–600, default 60.</div>
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
              <div class="row-label">Connect timeout (seconds)</div>
              <div class="row-sub">Maximum TCP connection establishment time. 3–60, default 10.</div>
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
              <div class="row-label">Download cache TTL (seconds)</div>
              <div class="row-sub">How long a downloaded vendor archive stays in memory for sibling DLLs that share it. 60–3600, default 300.</div>
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
      </section>
    {/if}
  </div>
  </div>
{/if}

<style>
  .view-header { margin-bottom: 18px; }
  .loading { padding: 60px 0; text-align: center; color: var(--text-muted); }

  .settings-hero {
    margin-bottom: 18px;
    padding: 16px 20px;
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 18px;
    align-items: center;
  }
  .hero-eyebrow {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--accent);
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
  }
  .hero-title-row { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
  .hero-version { font-size: var(--fs-xl); font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tighter); font-variant-numeric: tabular-nums; }
  .hero-status { padding: 4px 10px; font-size: var(--fs-2xs); }
  .hero-sub { font-size: var(--fs-sm); color: var(--text-secondary); margin-top: 6px; line-height: var(--lh-snug); max-width: 540px; }
  .hero-actions { display: flex; gap: 6px; flex-wrap: wrap; }

  .settings-layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 28px;
    align-items: start;
  }
  @media (max-width: 900px) {
    .settings-layout { grid-template-columns: 1fr; }
  }
  .side-nav {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 0;
  }
  .side-tab {
    position: relative;
    display: grid;
    grid-template-columns: 22px 1fr;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    font-weight: 500;
    text-align: left;
    border-radius: var(--radius-md);
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
  }
  .side-tab:hover { background: var(--bg-elevated); color: var(--text-primary); }
  .side-tab-icon { color: var(--text-muted); display: inline-flex; }
  .side-tab.active { background: var(--accent-dim); color: var(--accent); }
  .side-tab.active .side-tab-icon { color: var(--accent); }
  .side-tab-rail {
    position: absolute;
    left: -6px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 60%;
    border-radius: var(--radius-xs);
    background: transparent;
    transition: background 0.12s var(--ease);
  }
  .side-tab.active .side-tab-rail { background: var(--accent); }

  .tab-panels { max-width: 920px; min-width: 0; }
  .section-head { margin-bottom: 10px; }
  .section-title-h {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
    margin-bottom: 3px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .section-tag {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    padding: 1px 7px;
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    font-weight: 600;
  }
  .section-help {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    line-height: var(--lh-snug);
  }
  .section-help.muted { font-size: var(--fs-xs); opacity: 0.75; margin-top: 4px; }

  .card { padding: 4px 18px; }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 18px;
    padding: 14px 0;
  }
  .art-row { align-items: flex-start; display: grid; grid-template-columns: 1fr 280px; gap: 16px; }
  .row-divider { border-top: 1px solid var(--border); }
  .inline-num {
    width: 80px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    text-align: right;
    flex-shrink: 0;
  }
  .inline-num:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .row-label { font-size: var(--fs-base); font-weight: 500; color: var(--text-primary); display: inline-flex; align-items: center; gap: 8px; }
  .row-sub { font-size: var(--fs-xs); color: var(--text-secondary); margin-top: 3px; line-height: 1.5; }
  .row-text { min-width: 0; flex: 1; }
  .files-disclosure {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 6px;
    font-size: var(--fs-2xs);
    font-weight: 500;
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 0;
  }
  .files-disclosure:hover { color: var(--text-primary); }
  .files-disclosure .chev { transition: transform 0.15s var(--ease); }
  .files-disclosure .chev.open { transform: rotate(180deg); }
  .files-meta { margin-top: 6px; padding: 7px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: var(--radius-sm); font-size: var(--fs-2xs); color: var(--text-muted); }

  .folder-input-row { display: flex; gap: 10px; align-items: center; padding: 14px 0 4px; }
  .folder-input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    border: 1px solid transparent;
    transition: border-color 0.15s var(--ease), background 0.15s var(--ease), box-shadow 0.15s var(--ease);
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
    gap: 8px;
    padding: 24px 0 18px;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .path-list { list-style: none; padding: 12px 0 4px; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .path-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    transition: background 0.12s var(--ease);
  }
  .path-row:hover { background: var(--bg-card-hover); }
  .path-icon { width: 32px; height: 32px; border-radius: 10px; }
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
  .path-action.path-action-danger:hover { background: var(--badge-red-bg); color: var(--badge-red-fg); }
  .row-actions { padding: 12px 0 14px; }

  .launcher-card { padding: 6px 18px; }
  .launcher-row {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 20px;
    align-items: flex-start;
    padding: 16px 0;
  }
  .launcher-head { display: flex; align-items: center; gap: 12px; min-width: 0; }
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
  .launcher-input-col { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .path-input-row { display: flex; align-items: stretch; gap: 6px; height: 36px; }
  .path-input-row input {
    flex: 1;
    min-width: 0;
    padding: 0 12px;
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    color: var(--text-primary);
    transition: border-color 0.12s var(--ease), background 0.12s var(--ease), box-shadow 0.12s var(--ease);
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
</style>
