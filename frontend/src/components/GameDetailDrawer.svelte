<script lang="ts">
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { t, locale, translate } from "../lib/i18n/index";
  import { setActiveArt, clearActiveArt } from "../lib/artContext";
  import { coverAccent } from "../lib/coverAccent";
  import { EXTERNAL_URLS, STREAMLINE_OVERRIDE_NOTE } from "../lib/ux";
  import {
    games,
    gameDlls,
    gameDlssEnabler,
    gameDllsLoading,
    gameDllErrors,
    catalogLatestByKey,
    relationContext,
    settings,
    persistSettings,
    showToast,
    optimisticToggle,
    rescanGame,
    driverReports,
  } from "../lib/stores";
  import { dllRelation, targetVersion, recordUpdatable, isStreamlinePlugin } from "../lib/relation";
  import { addBlacklistEntry, removeBlacklistEntry, findGameExecutable, detectAnticheat, saveSettings, type AppSettings, type DllRecord, type AntiCheatReport } from "../lib/api";
  import { hasAntiCheat, statusNote, warningMessage, severity, detectedNames } from "../lib/anticheat";
  import DlssOverridePanel from "./DlssOverridePanel.svelte";
  import { dispatchApply, dispatchStreamlineSet, type ApplyTarget } from "../lib/applyController";
  import {
    LAUNCHER_ACCENTS,
    familyLabel,
    familyShort,
    familyCatalogKey,
    launcherLabel,
    recordFeature,
    featureTitle,
    featureIconId,
    featureVendor,
    FEATURE_ORDER,
    GROUP_ACCENT,
    VENDOR_ACCENTS,
    filenameFromPath,
    type FeatureSlot,
  } from "../lib/labels";
  import VersionPickerPopover from "./VersionPickerPopover.svelte";
  import FeatureIcon from "./FeatureIcon.svelte";
  import ContextMenu, { type ContextMenuAction } from "./ContextMenu.svelte";

  let { gameId, onClose, onApplyStart }: {
    gameId: string;
    onClose: () => void;
    onApplyStart: () => void;
  } = $props();

  let game = $derived($games.find((g) => g.id === gameId));
  let coverAccentColor = $state<string | null>(null);
  $effect(() => {
    if (game?.image_url) setActiveArt(game.image_url);
  });
  $effect(() => {
    const url = game?.image_url;
    coverAccentColor = null;
    if (!url) return;
    let active = true;
    void coverAccent(url).then((color) => {
      if (active) coverAccentColor = color;
    });
    return () => {
      active = false;
    };
  });

  onDestroy(() => {
    clearActiveArt();
  });

  let records: DllRecord[] = $derived($gameDlls[gameId] ?? []);
  let dlssEnabler = $derived($gameDlssEnabler[gameId] ?? false);
  let loading = $derived($gameDllsLoading[gameId] ?? false);
  let scanError = $derived($gameDllErrors[gameId] ?? null);
  let imgErrored = $state(false);
  let rescanning = $state(false);

  async function doRescan(): Promise<void> {
    if (rescanning) return;
    rescanning = true;
    try {
      await rescanGame(gameId);
      if (!scanError) showToast("success", translate(get(locale), "component.gameDrawer.toast.rescanComplete"));
      else showToast("danger", translate(get(locale), "component.gameDrawer.toast.rescanFailed", { error: scanError }));
    } finally {
      rescanning = false;
    }
  }

  let pref = $derived($settings?.game_preferences[gameId]);
  let disabledFamilies: string[] = $derived(pref?.disabled_families ?? []);
  let pinnedVersions: Record<string, string> = $derived(pref?.pinned_versions ?? {});


  let selected = $state<Record<string, boolean>>({});
  let activeGameId = $state<string | null>(null);
  let pickerOpenFor = $state<string | null>(null);
  let expandedFeatures = $state<Record<string, boolean>>({});
  let advancedExpanded = $state(false);

  let dlssExpanded = $state(false);
  let gameExe = $state<string | null>(null);
  let exeResolving = $state(false);
  let exeResolved = $state(false);

  let nvidiaPacked = $derived.by(
    () => $driverReports.find((r) => r.device.vendor === "nvidia")?.installed.packed ?? 0,
  );

  async function resolveExe(): Promise<void> {
    if (!game || exeResolved) return;
    exeResolving = true;
    try {
      gameExe = await findGameExecutable(game.install_dir);
    } catch {
      gameExe = null;
    } finally {
      exeResolving = false;
      exeResolved = true;
    }
  }

  function toggleDlss(): void {
    dlssExpanded = !dlssExpanded;
    if (dlssExpanded) void resolveExe();
  }

  $effect(() => {
    if (gameId !== activeGameId) {
      activeGameId = gameId;
      const next: Record<string, boolean> = {};
      for (const r of records) {
        const key = rowKey(r);
        next[key] = isOutdated(r) && !disabledFamilies.includes(r.family);
      }
      selected = next;
      expandedFeatures = {};
      dlssExpanded = false;
      gameExe = null;
      exeResolved = false;
      acReport = null;
      void loadAntiCheat();
    }
  });

  function rowKey(r: DllRecord): string {
    return `${r.family}|${r.path}`;
  }

  function isOutdated(r: DllRecord): boolean {
    if (!recordUpdatable(r, $settings?.update_prefs ?? null)) return false;
    return relation(r) === "outdated";
  }

  function targetFor(r: DllRecord): string | null {
    const pin = pinnedVersions[rowKey(r)] ?? null;
    return targetVersion(r, $relationContext, pin);
  }

  function latestFor(r: DllRecord): string | null {
    return $catalogLatestByKey[familyCatalogKey(r.family)] ?? null;
  }

  function relation(r: DllRecord): "outdated" | "same" | "ahead" | "no-target" {
    const pin = pinnedVersions[rowKey(r)] ?? null;
    return dllRelation(r, $relationContext, pin);
  }

  async function toggleFeatureDisabled(recs: DllRecord[]): Promise<void> {
    if (!$settings) return;
    const before: AppSettings = $settings;
    const families: Set<string> = new Set(recs.map((r) => r.family));
    const wasAllDisabled = [...families].every((f) => disabledFamilies.includes(f));
    const prefs = { ...before.game_preferences };
    const cur = prefs[gameId] ?? { disabled_families: [], pinned_versions: {} };
    let next = [...cur.disabled_families];
    if (wasAllDisabled) {
      next = next.filter((f) => !families.has(f));
    } else {
      for (const f of families) if (!next.includes(f)) next.push(f);
    }
    prefs[gameId] = { ...cur, disabled_families: next };
    const after: AppSettings = { ...before, game_preferences: prefs };
    const loc = get(locale);
    const featureName = recs.length > 0 ? familyShort(recs[0].family) : translate(loc, "component.gameDrawer.featureFallback");
    await optimisticToggle({
      applyOptimistic: () => settings.set(after),
      revert: () => settings.set(before),
      commit: () => saveSettings(after),
      message: translate(loc, wasAllDisabled ? "component.gameDrawer.toast.featureEnabled" : "component.gameDrawer.toast.featureDisabled", {
        feature: featureName,
        game: game?.name ?? translate(loc, "component.gameDrawer.thisGame"),
      }),
    });
  }

  async function setPin(key: string, version: string | null): Promise<void> {
    if (!$settings) return;
    const prefs = { ...$settings.game_preferences };
    const cur = prefs[gameId] ?? { disabled_families: [], pinned_versions: {} };
    const pins = { ...cur.pinned_versions };
    if (version === null) {
      delete pins[key];
    } else {
      pins[key] = version;
    }
    prefs[gameId] = { ...cur, pinned_versions: pins };
    await persistSettings({ ...$settings, game_preferences: prefs });
  }

  type FeatureBucket = {
    feature: FeatureSlot;
    records: DllRecord[];
    primary: DllRecord;
    title: string;
    blurb: string;
    iconId: string;
    accent: string;
    anyOutdated: boolean;
    anyAhead: boolean;
    allUpToDate: boolean;
    allDisabled: boolean;
    statusLabel: string;
    statusTone: "update" | "success" | "info" | "neutral";
  };

  function pickPrimary(feature: FeatureSlot, recs: DllRecord[]): DllRecord {
    if (feature === "dlss_sr") {
      const r = recs.find((x) => x.family === "dlss_sr") ?? recs.find((x) => x.family === "streamline" && filenameFromPath(x.path).toLowerCase().endsWith("sl.dlss.dll"));
      if (r) return r;
    }
    if (feature === "dlss_fg") {
      const r = recs.find((x) => x.family === "dlss_fg") ?? recs.find((x) => x.family === "streamline" && filenameFromPath(x.path).toLowerCase().endsWith("sl.dlss_g.dll"));
      if (r) return r;
    }
    if (feature === "dlss_rr") {
      const r = recs.find((x) => x.family === "dlss_rr") ?? recs.find((x) => x.family === "streamline" && filenameFromPath(x.path).toLowerCase().endsWith("sl.dlss_d.dll"));
      if (r) return r;
    }
    if (feature === "fsr_upscaler") {
      const r = recs.find((x) => x.family === "fsr_upscaler") ?? recs.find((x) => x.family === "fsr_upscaler_vk") ?? recs.find((x) => x.family === "fsr_loader");
      if (r) return r;
    }
    if (feature === "xess_sr") {
      const r = recs.find((x) => x.family === "xess_sr") ?? recs.find((x) => x.family === "xess_sr_dx11") ?? recs.find((x) => x.family === "xell");
      if (r) return r;
    }
    return recs[0];
  }

  let featureBuckets = $derived.by<FeatureBucket[]>(() => {
    const map = new Map<FeatureSlot, DllRecord[]>();
    for (const r of records) {
      const f = recordFeature(r);
      if (!map.has(f)) map.set(f, []);
      map.get(f)!.push(r);
    }
    const out: FeatureBucket[] = [];
    for (const fid of FEATURE_ORDER) {
      const recs = map.get(fid);
      if (!recs || recs.length === 0) continue;
      const primary = pickPrimary(fid, recs);
      const anyOutdated = recs.some((r) => relation(r) === "outdated");
      const anyAhead = recs.some((r) => relation(r) === "ahead");
      const inCatalog = recs.filter((r) => relation(r) !== "no-target");
      const allUpToDate = inCatalog.length > 0 && inCatalog.every((r) => relation(r) === "same");
      const allDisabled = recs.every((r) => disabledFamilies.includes(r.family));
      const tr = $t;
      let label = tr("component.gameDrawer.status.notInCatalog");
      let tone: FeatureBucket["statusTone"] = "neutral";
      if (allDisabled) { label = tr("component.gameDrawer.status.disabled"); tone = "neutral"; }
      else if (anyOutdated) { label = recs.length > 1 ? tr("component.gameDrawer.status.updatesReady") : tr("component.gameDrawer.status.updateReady"); tone = "update"; }
      else if (anyAhead) { label = tr("component.gameDrawer.status.aheadOfCatalog"); tone = "info"; }
      else if (allUpToDate) { label = tr("status.up_to_date"); tone = "success"; }
      out.push({
        feature: fid,
        records: recs,
        primary,
        title: featureTitle(fid),
        blurb: $t("feature." + fid + ".blurb"),
        iconId: featureIconId(fid),
        accent: VENDOR_ACCENTS[featureVendor(fid)] ?? "#94a3b8",
        anyOutdated,
        anyAhead,
        allUpToDate,
        allDisabled,
        statusLabel: label,
        statusTone: tone,
      });
    }
    return out;
  });

  let advancedRecords = $derived(records.filter((r) => recordFeature(r) === "advanced"));

  type AdvancedRow = { family: string; label: string; records: DllRecord[]; primary: DllRecord; anyOutdated: boolean; };
  let advancedRows = $derived.by<AdvancedRow[]>(() => {
    const map = new Map<string, DllRecord[]>();
    for (const r of advancedRecords) {
      const key = r.family === "streamline" ? `streamline:${filenameFromPath(r.path).toLowerCase()}` : r.family;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(r);
    }
    const out: AdvancedRow[] = [];
    for (const [key, recs] of map) {
      const fname = filenameFromPath(recs[0].path);
      const label = key.startsWith("streamline:")
        ? `Streamline · ${fname.replace(/\.dll$/i, "").replace(/^sl\./i, "").replace(/_/g, " ")}`
        : familyLabel(recs[0].family);
      out.push({ family: key, label, records: recs, primary: recs[0], anyOutdated: recs.some((r) => relation(r) === "outdated") });
    }
    return out.sort((a, b) => a.label.localeCompare(b.label));
  });

  function selectAllOutdated(): void {
    const next: Record<string, boolean> = { ...selected };
    for (const r of records) {
      if (isOutdated(r) && !disabledFamilies.includes(r.family)) {
        next[rowKey(r)] = true;
      }
    }
    selected = next;
  }

  function clearSelection(): void {
    selected = {};
  }

  function toggleFeatureSelection(bucket: FeatureBucket, checked: boolean): void {
    const next = { ...selected };
    for (const r of bucket.records) {
      if (disabledFamilies.includes(r.family)) continue;
      const rel = relation(r);
      if (rel === "same" || rel === "no-target") continue;
      next[rowKey(r)] = checked;
    }
    selected = next;
  }

  function featureSelectionState(bucket: FeatureBucket): "all" | "some" | "none" {
    const eligible = bucket.records.filter((r) => !disabledFamilies.includes(r.family) && relation(r) !== "same" && relation(r) !== "no-target");
    if (eligible.length === 0) return "none";
    const sel = eligible.filter((r) => selected[rowKey(r)]).length;
    if (sel === 0) return "none";
    if (sel === eligible.length) return "all";
    return "some";
  }

  function selectedRecords(): { record: DllRecord; target: string }[] {
    const out: { record: DllRecord; target: string }[] = [];
    for (const r of records) {
      if (!selected[rowKey(r)]) continue;
      if (disabledFamilies.includes(r.family)) continue;
      const tgt = targetFor(r);
      if (!tgt) continue;
      out.push({ record: r, target: tgt });
    }
    return out;
  }

  let acConfirming = $state(false);

  $effect(() => {
    if (gameId) acConfirming = false;
  });

  function requestApply(): void {
    if (selectedCount === 0) return;
    if (acActive && acSeverity === "danger" && !acConfirming) {
      acConfirming = true;
      return;
    }
    acConfirming = false;
    void applySelected();
  }

  function cancelApplyConfirm(): void {
    acConfirming = false;
  }

  async function applySelected(): Promise<void> {
    if (!game) return;
    const items = selectedRecords();
    if (items.length === 0) {
      showToast("warning", translate(get(locale), "component.gameDrawer.toast.nothingSelected"));
      return;
    }
    const game_label = `${launcherLabel(game.launcher)} - ${game.name}`;
    const targets: ApplyTarget[] = items.map((it) => ({
      game_id: game!.id,
      game_label,
      record: it.record,
      target_version: it.target,
    }));
    await dispatchApply(targets, { showModal: onApplyStart });
    try {
      await rescanGame(game.id);
      selected = {};
    } catch (err: unknown) {
      showToast(
        "warning",
        translate(get(locale), "component.gameDrawer.toast.rescanAfterApplyFailed", { error: String(err) }),
      );
    }
  }

  let streamlineSetMembers = $derived(
    records.filter((r) => isStreamlinePlugin(filenameFromPath(r.path)) && isOutdated(r)),
  );
  let streamlineSetTarget = $derived(
    streamlineSetMembers.length ? targetFor(streamlineSetMembers[0]) : null,
  );

  async function applyStreamlineSetAction(): Promise<void> {
    if (!game || streamlineSetMembers.length === 0) return;
    const game_label = `${launcherLabel(game.launcher)} - ${game.name}`;
    const targets: ApplyTarget[] = [];
    for (const r of streamlineSetMembers) {
      const tgt = targetFor(r);
      if (!tgt) continue;
      targets.push({ game_id: game.id, game_label, record: r, target_version: tgt });
    }
    await dispatchStreamlineSet(targets, { showModal: onApplyStart });
    try {
      await rescanGame(game.id);
      selected = {};
    } catch (err: unknown) {
      showToast(
        "warning",
        translate(get(locale), "component.gameDrawer.toast.rescanAfterApplyFailed", { error: String(err) }),
      );
    }
  }

  let rowMenu = $state<{ x: number; y: number; primaryKey: string } | null>(null);
  let rowMenuItems = $derived([
    { action: "open_folder" as ContextMenuAction, label: $t("view.library.menu.openFolder") },
    { action: "scan" as ContextMenuAction, label: $t("component.gameDrawer.menu.rescan") },
    { action: "pin" as ContextMenuAction, label: $t("view.library.menu.pin") },
    { action: "hide" as ContextMenuAction, label: $t("component.gameDrawer.menu.hideGame") },
  ]);

  function openRowMenu(primaryKey: string, e: MouseEvent): void {
    e.preventDefault();
    rowMenu = { x: e.clientX, y: e.clientY, primaryKey };
  }

  async function onRowMenuSelect(action: ContextMenuAction): Promise<void> {
    const key = rowMenu?.primaryKey ?? null;
    switch (action) {
      case "open_folder":
        await openFolder();
        break;
      case "scan":
        await doRescan();
        break;
      case "pin":
        if (key) pickerOpenFor = key;
        break;
      case "hide":
        await toggleHidden();
        break;
    }
  }

  async function openFolder(): Promise<void> {
    if (!game) return;
    try {
      const { openPath } = await import("../lib/api");
      await openPath(game.install_dir);
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "component.gameDrawer.toast.openFolderFailed", { error: String(err) }));
    }
  }

  let isHidden = $derived(($settings?.blacklist ?? []).includes(gameId));

  async function toggleHidden(): Promise<void> {
    if (!game) return;
    const wasHidden = isHidden;
    const loc = get(locale);
    try {
      const next = wasHidden ? await removeBlacklistEntry(game.id) : await addBlacklistEntry(game.id);
      if ($settings) settings.set({ ...$settings, blacklist: next });
      showToast(
        wasHidden ? "success" : "info",
        translate(loc, wasHidden ? "view.library.toast.gameRestored" : "view.library.toast.gameHidden", { name: game.name }),
      );
      if (!wasHidden) onClose();
    } catch (err: unknown) {
      showToast("danger", translate(loc, wasHidden ? "view.library.toast.restoreFailed" : "view.library.toast.hideFailed", { error: String(err) }));
    }
  }

  let outdatedCount = $derived(
    records.filter(isOutdated).filter((r) => !disabledFamilies.includes(r.family)).length,
  );
  let selectedCount = $derived(Object.values(selected).filter(Boolean).length);
  let aheadCount = $derived(
    records.filter((r) => selected[rowKey(r)] && relation(r) === "ahead").length,
  );
  let accent = $derived(game ? LAUNCHER_ACCENTS[game.launcher] ?? "#22d3ee" : "#22d3ee");

  let acReport = $state<AntiCheatReport | null>(null);
  async function loadAntiCheat(): Promise<void> {
    if (!game) return;
    try {
      acReport = await detectAnticheat(game.install_dir, game.app_id, game.name);
    } catch {
      acReport = null;
    }
  }

  let acActive = $derived(hasAntiCheat(acReport));
  let acStatus = $derived(acReport ? statusNote(acReport) : null);
  let acSeverity = $derived(acReport ? severity(acReport) : "warning");
  let acNames = $derived(acReport ? detectedNames(acReport) : "");
  let acLearnUrl = $derived(acReport?.source_url ?? EXTERNAL_URLS.anticheatFaq);
</script>

<svelte:window onkeydown={(e) => { if (game && e.key === "Escape") onClose(); }} />

{#if game}
  <div class="detail-view" style:--launcher-accent={accent} style:--game-accent={coverAccentColor ?? "var(--accent)"} aria-label={game.name}>
    <button class="detail-back" onclick={onClose} title={$t("component.gameDrawer.backToLibraryTitle")} aria-label={$t("component.gameDrawer.backToLibrary")}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
    </button>
    <header class="detail-hero">
      <div class="drawer-art">
        {#if game.image_url && !imgErrored}
          <img src={game.image_url} alt={game.name} onerror={() => (imgErrored = true)} />
        {:else}
          <div class="drawer-art-fallback">{game.name.slice(0, 1).toUpperCase()}</div>
        {/if}
        <div class="drawer-art-overlay"></div>
      </div>
      <div class="drawer-meta">
        <span class="launcher-chip">{launcherLabel(game.launcher)}</span>
        <h2 class="drawer-title">{game.name}</h2>
        <p class="drawer-path mono truncate" title={game.install_dir}>{game.install_dir}</p>
      </div>
    </header>

    <div
      class="status-ribbon"
      class:is-update={!loading && !scanError && outdatedCount > 0}
      class:is-success={!loading && !scanError && records.length > 0 && outdatedCount === 0}
      class:is-danger={!!scanError}
      class:is-muted={loading || (!scanError && records.length === 0)}
      aria-live="polite"
    >
      {#if loading || rescanning}
        <span class="ribbon-dot is-pulse"></span>
        <span>{$t("component.gameDrawer.ribbon.scanning")}</span>
      {:else if scanError}
        <span class="ribbon-dot"></span>
        <span>{$t("component.gameDrawer.ribbon.scanFailed")} <span class="mono">{scanError}</span></span>
      {:else if records.length === 0}
        <span class="ribbon-dot"></span>
        <span>{$t("component.gameDrawer.ribbon.noDlls")}</span>
      {:else if outdatedCount === 0}
        <span class="ribbon-dot"></span>
        <span>{$t("component.gameDrawer.ribbon.allUpToDate", { count: records.length })}</span>
      {:else}
        <span class="ribbon-dot is-pulse"></span>
        <span>{$t("component.gameDrawer.ribbon.updatesReady", { count: outdatedCount })}{aheadCount > 0 ? $t("component.gameDrawer.ribbon.aheadSuffix", { count: aheadCount }) : ""}</span>
      {/if}
    </div>

    <div class="drawer-body">
      {#if acActive}
        <div class="warning-banner edge-accent" class:is-warning={acSeverity !== "danger"} class:is-danger={acSeverity === "danger"} role="alert">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          <span class="warning-text">{warningMessage(acReport!)}{#if acStatus} {acStatus}{/if}</span>
          <button
            class="learn-more"
            title={$t("component.gameDrawer.anticheat.learnMoreTitle")}
            onclick={async () => {
              try {
                const { open } = await import("@tauri-apps/plugin-shell");
                await open(acLearnUrl);
              } catch (err) { showToast("warning", translate(get(locale), "component.gameDrawer.toast.openLinkFailed", { error: String(err) })); }
            }}
          >{$t("component.gameDrawer.anticheat.learnMore")}</button>
        </div>
      {/if}

      {#if dlssEnabler}
        <div class="warning-banner edge-accent is-info" role="status">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
          <span class="warning-text">{$t("note.enablerManaged")}</span>
        </div>
      {/if}

      {#if loading || rescanning}
        <div class="loading-state">
          <span class="spinner"></span>
          <span>{rescanning ? $t("component.gameDrawer.loading.rescanning") : $t("component.gameDrawer.loading.scanning")} — {game.install_dir.split(/[\\/]/).pop()}</span>
        </div>
      {:else if scanError}
        <div class="empty-state error-state">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
          <h3>{$t("component.gameDrawer.scanError.title")}</h3>
          <p class="error-msg mono">{scanError}</p>
          <p class="error-hint">{$t("component.gameDrawer.scanError.hint")}</p>
          <button class="btn btn-primary" onclick={doRescan}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
            {$t("component.gameDrawer.scanError.retry")}
          </button>
        </div>
      {:else if records.length === 0}
        <div class="empty-state">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <h3>{$t("component.gameDrawer.empty.title")}</h3>
          <p>{$t("component.gameDrawer.empty.body")}</p>
          <div class="empty-actions">
            <button class="btn btn-accent" onclick={doRescan}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
              {$t("view.library.rescan")}
            </button>
            <button class="btn btn-ghost" onclick={openFolder}>{$t("view.library.menu.openFolder")}</button>
          </div>
        </div>
      {:else}
        <div class="summary-row">
          <div class="summary-stat">
            <span class="stat-num">{records.length}</span>
            <span class="stat-label">{$t("component.gameDrawer.stat.files")}</span>
          </div>
          <div class="summary-stat">
            <span class="stat-num" class:is-update={outdatedCount > 0}>{outdatedCount}</span>
            <span class="stat-label">{$t("component.gameDrawer.stat.updates")}</span>
          </div>
          <div class="summary-stat">
            <span class="stat-num" class:is-accent={selectedCount > 0}>{selectedCount}</span>
            <span class="stat-label">{$t("component.gameDrawer.stat.selected")}</span>
          </div>
        </div>

        {#if outdatedCount > 0}
          <div class="quick-actions">
            <button class="btn btn-sm btn-accent" onclick={selectAllOutdated}>
              {$t("component.gameDrawer.selectAllUpdates", { count: outdatedCount })}
            </button>
            <button class="btn btn-sm btn-ghost" onclick={clearSelection} disabled={selectedCount === 0}>
              {$t("component.gameDrawer.clearSelection")}
            </button>
          </div>
        {/if}

        {#if featureBuckets.length > 0}
          <ul class="feature-list stagger">
            {#each featureBuckets as b (b.feature)}
              {@const selState = featureSelectionState(b)}
              {@const expanded = !!expandedFeatures[b.feature]}
              {@const primaryKey = rowKey(b.primary)}
              {@const primaryRel = relation(b.primary)}
              {@const primaryTarget = targetFor(b.primary)}
              {@const primaryLatest = latestFor(b.primary)}
              {@const primaryPinned = pinnedVersions[primaryKey]}
              {@const primaryAside = primaryRel === "ahead" || (primaryRel === "same" && primaryTarget != null && primaryTarget !== (b.primary.current_version ?? ""))}
              <li class="feature-row" class:is-update={b.anyOutdated && !b.allDisabled} class:disabled={b.allDisabled} oncontextmenu={(e) => openRowMenu(primaryKey, e)}>
                <label class="feature-check" title={selState === "all" ? $t("component.gameDrawer.feature.deselectAll") : $t("component.gameDrawer.feature.selectAll")}>
                  <input
                    type="checkbox"
                    checked={selState !== "none"}
                    indeterminate={selState === "some"}
                    disabled={b.allDisabled || !b.anyOutdated}
                    onchange={(e) => toggleFeatureSelection(b, (e.target as HTMLInputElement).checked)}
                  />
                  <span class="check-box"></span>
                </label>
                <div class="feature-glyph" style:--feature-accent={b.accent} aria-hidden="true">
                  <FeatureIcon id={b.iconId} size={20} />
                </div>
                <div class="feature-body">
                  <div class="feature-head">
                    <span class="feature-title">{b.title}</span>
                    {#if b.statusTone === "update"}
                      <span class="chip chip-update">{b.statusLabel}</span>
                    {:else if b.statusTone === "success"}
                      <span class="chip chip-success">{b.statusLabel}</span>
                    {:else if b.statusTone === "info"}
                      <span class="chip chip-info">{b.statusLabel}</span>
                    {:else}
                      <span class="chip chip-neutral">{b.statusLabel}</span>
                    {/if}
                  </div>
                  <p class="feature-blurb">{b.blurb}</p>
                  <div class="feature-versions">
                    <span class="ver-pair">
                      <span class="ver current" class:is-update={primaryRel === "outdated"}>v{b.primary.current_version ?? "?"}</span>
                      {#if primaryAside}
                        <button class="ver catalog-aside" onclick={() => (pickerOpenFor = primaryKey)} title={$t("component.gameDrawer.version.pickDifferent")}>
                          <span class="muted">{$t("component.gameDrawer.version.catalogAside", { version: primaryTarget ?? "" })}</span>
                        </button>
                      {:else}
                        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                        <button class="ver target-btn" onclick={() => (pickerOpenFor = primaryKey)} title={$t("component.gameDrawer.version.choose")}>
                          {#if primaryTarget}
                            <span class="target">v{primaryTarget}</span>
                            {#if primaryPinned && primaryPinned !== primaryLatest}
                              <span class="chip chip-update pin-chip" title={$t("component.gameDrawer.version.pinnedTitle")}>{$t("component.gameDrawer.version.pinned")}</span>
                            {/if}
                          {:else}
                            <span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>
                          {/if}
                          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                        </button>
                      {/if}
                    </span>
                    {#if b.records.length > 1}
                      <button class="files-toggle" onclick={() => (expandedFeatures = { ...expandedFeatures, [b.feature]: !expanded })} aria-expanded={expanded}>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                        {expanded ? $t("component.gameDrawer.files.hideMany") : $t("component.gameDrawer.files.showMany", { count: b.records.length })}
                      </button>
                    {:else}
                      <button class="files-toggle subtle" onclick={() => (expandedFeatures = { ...expandedFeatures, [b.feature]: !expanded })} aria-expanded={expanded} title={filenameFromPath(b.primary.path)}>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                        {expanded ? $t("component.gameDrawer.files.hideOne") : $t("component.gameDrawer.files.showOne")}
                      </button>
                    {/if}
                  </div>
                </div>
                <button class="feature-eye" onclick={() => void toggleFeatureDisabled(b.records)} title={b.allDisabled ? $t("component.gameDrawer.feature.reEnable") : $t("component.gameDrawer.feature.disable")} aria-label={$t("component.gameDrawer.feature.toggleAria")}>
                  {#if b.allDisabled}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                  {:else}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  {/if}
                </button>
                {#if expanded}
                  <ul class="files-list">
                    {#each b.records as r (r.path)}
                      {@const k = rowKey(r)}
                      {@const tgt = targetFor(r)}
                      {@const lat = latestFor(r)}
                      {@const rel = relation(r)}
                      {@const fd = disabledFamilies.includes(r.family)}
                      {@const pin = pinnedVersions[k]}
                      {@const fileAside = rel === "ahead" || (rel === "same" && tgt != null && tgt !== (r.current_version ?? ""))}
                      <li class="file-row" class:disabled={fd}>
                        <label class="file-check">
                          <input
                            type="checkbox"
                            checked={selected[k] ?? false}
                            disabled={fd || rel === "same" || rel === "no-target"}
                            onchange={(e) => (selected = { ...selected, [k]: (e.target as HTMLInputElement).checked })}
                          />
                          <span class="check-box"></span>
                        </label>
                        <div class="file-info">
                          <div class="file-top">
                            <span class="file-name mono">{filenameFromPath(r.path)}</span>
                            <span class="file-tag">{familyShort(r.family)}</span>
                          </div>
                          <div class="file-versions">
                            <span class="ver current mono" class:is-update={rel === "outdated"}>v{r.current_version ?? "?"}</span>
                            {#if fileAside}
                              <button class="ver catalog-aside small" onclick={() => (pickerOpenFor = k)} title={$t("component.gameDrawer.version.pickDifferent")}>
                                <span class="muted mono">{$t("component.gameDrawer.version.catalogAside", { version: tgt ?? "" })}</span>
                              </button>
                            {:else}
                              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                              <button class="ver target-btn small" onclick={() => (pickerOpenFor = k)}>
                                {#if tgt}
                                  <span class="target mono">v{tgt}</span>
                                  {#if pin && pin !== lat}<span class="chip chip-update pin-chip">{$t("component.gameDrawer.version.pinned")}</span>{/if}
                                {:else}
                                  <span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>
                                {/if}
                                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                              </button>
                            {/if}
                          </div>
                          <div class="file-path mono truncate" title={r.path}>{r.path}</div>
                        </div>
                        <div class="file-status">
                          {#if rel === "outdated"}
                            <span class="chip chip-update small-chip">{$t("component.gameDrawer.fileStatus.update")}</span>
                          {:else if rel === "ahead"}
                            <span class="chip chip-info small-chip" title={$t("component.gameDrawer.fileStatus.aheadTitle")}>{$t("component.gameDrawer.fileStatus.ahead")}</span>
                          {:else if rel === "same"}
                            <span class="chip chip-success small-chip">{$t("component.gameDrawer.fileStatus.current")}</span>
                          {:else}
                            <span class="chip chip-neutral small-chip">{$t("component.gameDrawer.status.notInCatalog")}</span>
                          {/if}
                        </div>
                        {#if pickerOpenFor === k}
                          <VersionPickerPopover
                            family={r.family}
                            filename={filenameFromPath(r.path)}
                            currentVersion={r.current_version}
                            latestVersion={lat}
                            pickedVersion={pin ?? null}
                            onPick={(v) => setPin(k, v)}
                            onClose={() => (pickerOpenFor = null)}
                          />
                        {/if}
                      </li>
                    {/each}
                  </ul>
                {/if}
                {#if pickerOpenFor === primaryKey}
                  <VersionPickerPopover
                    family={b.primary.family}
                    filename={filenameFromPath(b.primary.path)}
                    currentVersion={b.primary.current_version}
                    latestVersion={primaryLatest}
                    pickedVersion={primaryPinned ?? null}
                    onPick={(v) => setPin(primaryKey, v)}
                    onClose={() => (pickerOpenFor = null)}
                  />
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        {#if advancedRows.length > 0}
          <section class="advanced-block" class:open={advancedExpanded}>
            <button type="button" class="advanced-head" onclick={() => (advancedExpanded = !advancedExpanded)} aria-expanded={advancedExpanded}>
              <span class="advanced-titles">
                <span class="advanced-name">
                  <span class="advanced-dot" style:background={GROUP_ACCENT.advanced}></span>
                  {$t("feature.advanced.title")}
                  <span class="chip chip-neutral small-chip count">{advancedRows.length}</span>
                </span>
                <span class="advanced-sub">{$t("feature.advanced.blurb")}</span>
              </span>
              <span class="advanced-chevron" class:open={advancedExpanded}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
              </span>
            </button>
            {#if advancedExpanded}
              <ul class="files-list flat">
                {#each advancedRows as ar (ar.family)}
                  {@const r = ar.primary}
                  {@const k = rowKey(r)}
                  {@const tgt = targetFor(r)}
                  {@const lat = latestFor(r)}
                  {@const rel = relation(r)}
                  {@const fd = disabledFamilies.includes(r.family)}
                  {@const pin = pinnedVersions[k]}
                  <li class="file-row" class:disabled={fd}>
                    <label class="file-check">
                      <input
                        type="checkbox"
                        checked={selected[k] ?? false}
                        disabled={fd || rel === "same" || rel === "no-target"}
                        onchange={(e) => (selected = { ...selected, [k]: (e.target as HTMLInputElement).checked })}
                      />
                      <span class="check-box"></span>
                    </label>
                    <div class="file-info">
                      <div class="file-top">
                        <span class="file-name">{ar.label}</span>
                        <span class="file-tag mono">{filenameFromPath(r.path)}</span>
                      </div>
                      <div class="file-versions">
                        <span class="ver current mono" class:is-update={rel === "outdated"}>v{r.current_version ?? "?"}</span>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                        <button class="ver target-btn small" onclick={() => (pickerOpenFor = k)}>
                          {#if tgt}<span class="target mono">v{tgt}</span>{:else}<span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>{/if}
                          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                        </button>
                      </div>
                    </div>
                    <div class="file-status">
                      {#if rel === "outdated"}
                        <span class="chip chip-update small-chip">{$t("component.gameDrawer.fileStatus.update")}</span>
                      {:else if rel === "ahead"}
                        <span class="chip chip-info small-chip" title={$t("component.gameDrawer.fileStatus.aheadTitle")}>{$t("component.gameDrawer.fileStatus.ahead")}</span>
                      {:else if rel === "same"}
                        <span class="chip chip-success small-chip">{$t("component.gameDrawer.fileStatus.current")}</span>
                      {:else}
                        <span class="chip chip-neutral small-chip">{$t("component.gameDrawer.status.notInCatalog")}</span>
                      {/if}
                    </div>
                    {#if pickerOpenFor === k}
                      <VersionPickerPopover
                        family={r.family}
                        filename={filenameFromPath(r.path)}
                        currentVersion={r.current_version}
                        latestVersion={lat}
                        pickedVersion={pin ?? null}
                        onPick={(v) => setPin(k, v)}
                        onClose={() => (pickerOpenFor = null)}
                      />
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </section>
        {/if}
      {/if}

      {#if game && !loading && !rescanning}
        <section class="advanced-block" class:open={dlssExpanded}>
          <button type="button" class="advanced-head" onclick={toggleDlss} aria-expanded={dlssExpanded}>
            <span class="advanced-titles">
              <span class="advanced-name">
                <span class="advanced-dot" style="background: var(--vendor-nvidia);"></span>
                {$t("view.drivers.dlssOverrides")}
                <span class="chip chip-neutral small-chip count">NVIDIA</span>
              </span>
              <span class="advanced-sub">{$t("component.gameDrawer.dlss.sub")}</span>
            </span>
            <span class="advanced-chevron" class:open={dlssExpanded}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
            </span>
          </button>
          {#if dlssExpanded}
            <div class="dlss-drawer-body">
              {#if exeResolving}
                <p class="advanced-sub">{$t("component.gameDrawer.dlss.locating")}</p>
              {:else if gameExe}
                <p class="advanced-sub mono truncate" title={gameExe}>{gameExe}</p>
                <DlssOverridePanel scope={{ scope: "per_game", executable_path: gameExe }} driverPacked={nvidiaPacked} />
              {:else}
                <p class="advanced-sub">{$t("component.gameDrawer.dlss.noExe")}</p>
              {/if}
            </div>
          {/if}
        </section>
      {/if}
    </div>

    <footer class="drawer-foot">
      <button class="btn btn-ghost foot-util" onclick={openFolder} title={$t("component.gameDrawer.foot.openFolderTitle")} aria-label={$t("view.library.menu.openFolder")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
      </button>
      <button class="btn btn-ghost foot-util" onclick={doRescan} title={$t("component.gameDrawer.foot.rescanTitle")} aria-label={$t("view.library.rescan")} disabled={loading || rescanning}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
      </button>
      <button class="btn btn-ghost foot-util" onclick={toggleHidden} title={isHidden ? $t("component.gameDrawer.foot.restore") : $t("component.gameDrawer.foot.hide")} aria-label={isHidden ? $t("component.gameDrawer.foot.restore") : $t("component.gameDrawer.foot.hide")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
      </button>
      {#if aheadCount > 0}
        <span class="chip chip-info ahead-chip">{$t("component.gameDrawer.foot.aheadChip", { count: aheadCount })}</span>
      {/if}
      {#if streamlineSetMembers.length > 0}
        <button
          class="btn btn-ghost foot-streamline"
          onclick={applyStreamlineSetAction}
          title={`${$t("component.gameDrawer.streamlineSet.title")} ${STREAMLINE_OVERRIDE_NOTE}`}
        >
          {streamlineSetTarget
            ? $t("component.gameDrawer.streamlineSet.labelVersion", { version: streamlineSetTarget, count: streamlineSetMembers.length })
            : $t("component.gameDrawer.streamlineSet.label", { count: streamlineSetMembers.length })}
        </button>
      {/if}
      {#if acActive && selectedCount > 0}
        <span
          id="ac-apply-risk-note"
          class="ac-apply-risk"
          class:is-warning={acSeverity !== "danger"}
          class:is-danger={acSeverity === "danger"}
          role="note"
          title={acNames ? $t("component.gameDrawer.anticheat.apply.chipTitle", { names: acNames }) : ""}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          <span>{acSeverity === "danger" ? $t("component.gameDrawer.anticheat.apply.chipBan") : $t("component.gameDrawer.anticheat.apply.chipRisk")}</span>
        </span>
      {/if}
      {#if acConfirming}
        <div class="ac-apply-confirm" role="alertdialog" aria-label={$t("component.gameDrawer.anticheat.apply.confirmAria")}>
          <p class="ac-confirm-text">
            {acNames
              ? $t("component.gameDrawer.anticheat.apply.confirmBody", { names: acNames })
              : $t("component.gameDrawer.anticheat.apply.confirmBodyGeneric")}
          </p>
          <div class="ac-confirm-actions">
            <button class="btn btn-sm btn-ghost ac-confirm-cancel" onclick={cancelApplyConfirm}>
              {$t("component.gameDrawer.anticheat.apply.confirmCancel")}
            </button>
            <button class="btn btn-sm btn-danger ac-confirm-proceed" onclick={requestApply}>
              {$t("component.gameDrawer.anticheat.apply.confirmProceed")}
            </button>
          </div>
        </div>
      {/if}
      <button
        class="btn btn-primary halo is-update foot-apply"
        class:is-active={selectedCount > 0}
        class:is-ac-danger={acActive && acSeverity === "danger"}
        disabled={selectedCount === 0}
        aria-describedby={acActive && selectedCount > 0 ? "ac-apply-risk-note" : undefined}
        onclick={requestApply}
      >
        {acConfirming
          ? $t("component.gameDrawer.anticheat.apply.applyAnyway")
          : $t("component.gameDrawer.applySelected", { count: selectedCount })}
      </button>
    </footer>
  </div>
  {#if rowMenu}
    <ContextMenu
      x={rowMenu.x}
      y={rowMenu.y}
      items={rowMenuItems}
      onSelect={(a) => void onRowMenuSelect(a)}
      onClose={() => (rowMenu = null)}
    />
  {/if}
{/if}

<style>
  .detail-view {
    container-type: inline-size;
    container-name: drawer;
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    gap: 0;
    background: var(--bg-card);
    border: none;
    border-radius: 0;
    overflow: hidden;
  }
  .detail-back {
    position: absolute;
    top: var(--space-3);
    left: var(--space-3);
    z-index: 5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-full);
    color: #fff;
    background: rgba(0, 0, 0, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.3);
    cursor: pointer;
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .detail-back:hover { background: rgba(0, 0, 0, 0.72); border-color: rgba(255, 255, 255, 0.5); transform: translateX(-1px); }
  .detail-back:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .detail-hero {
    flex-shrink: 0;
    position: relative;
    border: none;
    border-radius: 0;
    overflow: hidden;
  }
  .drawer-art { width: 100%; height: clamp(132px, 20vh, 190px); overflow: hidden; position: relative; }
  .drawer-art::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--game-accent, var(--launcher-accent, var(--accent)));
    z-index: 2;
    pointer-events: none;
  }
  .drawer-art img { width: 100%; height: 100%; object-fit: cover; }
  .drawer-art-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-art-fallback);
    color: var(--accent);
    font-size: var(--fs-display);
    font-weight: 700;
    opacity: 0.55;
  }
  .drawer-art-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(0, 0, 0, 0.12) 22%,
      rgba(0, 0, 0, 0) 44%,
      rgba(0, 0, 0, 0.58) 72%,
      rgba(0, 0, 0, 0.92) 100%
    );
    pointer-events: none;
  }
  .drawer-meta {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: var(--space-4) var(--space-5) var(--space-4);
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .launcher-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px var(--space-2);
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
    background: var(--accent);
    color: var(--accent-fg);
  }
  .drawer-title {
    font-size: var(--fs-xl-plus);
    font-weight: 700;
    line-height: var(--lh-tight);
    letter-spacing: var(--letter-tighter);
    color: #fff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.6), 0 2px 12px rgba(0, 0, 0, 0.8);
  }
  .drawer-path {
    font-size: var(--fs-xs);
    color: rgba(255, 255, 255, 0.88);
    max-width: 100%;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
  }

  .drawer-body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--space-4) var(--space-4) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .drawer-body::-webkit-scrollbar { display: none; width: 0; height: 0; }

  .warning-banner {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    column-gap: var(--space-3);
    row-gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--warning-dim);
    border: 1px solid var(--warning);
    color: var(--warning);
    font-size: var(--fs-sm);
    line-height: var(--lh-snug);
  }
  .warning-banner.is-danger {
    background: var(--danger-dim);
    border-color: var(--danger);
    color: var(--danger);
  }
  .warning-banner.is-info {
    background: var(--info-dim);
    border-color: var(--info);
    color: var(--info);
  }
  .warning-banner svg { flex-shrink: 0; margin-top: 2px; }
  .warning-text {
    flex: 1 1 0;
    min-width: 0;
    overflow-wrap: anywhere;
    white-space: normal;
  }
  .learn-more {
    margin-left: auto;
    height: 28px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-cap);
    color: currentColor;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: var(--letter-wide);
    border: 1px solid currentColor;
    flex: 0 0 auto;
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .learn-more:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .learn-more:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .status-ribbon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 11px 16px;
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 0;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    background: var(--bg-card);
    font-variant-numeric: tabular-nums;
  }
  .status-ribbon.is-update { color: var(--update); background: var(--update-dim); }
  .status-ribbon.is-success { color: var(--success); background: var(--success-dim); }
  .status-ribbon.is-danger { color: var(--danger); background: var(--danger-dim); }
  .status-ribbon.is-muted { color: var(--text-muted); }
  .ribbon-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 6px currentColor;
    flex-shrink: 0;
  }
  .ribbon-dot.is-pulse { animation: pulse 2s var(--ease) infinite; }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 48px 20px 36px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty-state h3 { color: var(--text-primary); font-size: var(--fs-lg); font-weight: 600; margin-top: 6px; }
  .empty-state p { font-size: 12px; max-width: 360px; line-height: 1.55; }
  .empty-state svg { opacity: 0.6; }
  .empty-state .btn { margin-top: 12px; }
  .empty-state .empty-actions { display: flex; gap: 8px; margin-top: 12px; }
  .empty-state.error-state svg { color: var(--danger); opacity: 0.85; }
  .empty-state.error-state h3 { color: var(--danger); }
  .empty-state .error-msg { color: var(--danger); background: var(--danger-dim); padding: 8px 12px; border-radius: var(--radius-md); font-size: 11px; max-width: 100%; overflow-wrap: anywhere; }
  .empty-state .error-hint { color: var(--text-secondary); }

  .summary-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
  }
  .summary-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-2);
    background: var(--bg-cap);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .stat-num {
    font-size: var(--fs-xl);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tighter);
    font-variant-numeric: tabular-nums;
    line-height: var(--lh-tight);
  }
  .stat-num.is-update { color: var(--update); }
  .stat-num.is-accent { color: var(--accent); }
  .stat-label {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }

  .quick-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }

  .feature-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
  .feature-row {
    position: relative;
    display: grid;
    grid-template-columns: 22px 36px 1fr auto;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-4);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .feature-row:hover { background: var(--bg-card-hover); border-color: var(--border-hover); }
  .feature-row.is-update {
    border-color: color-mix(in srgb, var(--update) 40%, var(--border));
    background: color-mix(in srgb, var(--update-dim) 50%, var(--bg-card));
  }
  .feature-row.is-update::before {
    content: "";
    position: absolute;
    left: 0;
    top: var(--space-3);
    bottom: var(--space-3);
    width: 3px;
    border-radius: 0 var(--radius-full) var(--radius-full) 0;
    background: var(--update);
  }
  .feature-row.disabled { opacity: 0.55; }

  .feature-check { display: inline-flex; cursor: pointer; padding-top: 3px; }
  .feature-check input { display: none; }
  .check-box {
    width: 16px;
    height: 16px;
    border: 1.5px solid var(--border-strong);
    border-radius: var(--radius-xs);
    display: inline-block;
    background: var(--bg-input);
    transition: background 0.15s var(--ease), border-color 0.15s var(--ease);
    position: relative;
  }
  .feature-check input:checked + .check-box,
  .file-check input:checked + .check-box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .feature-check input:checked + .check-box::after,
  .file-check input:checked + .check-box::after {
    content: '';
    position: absolute;
    left: 4px;
    top: 1px;
    width: 5px;
    height: 9px;
    border: solid var(--accent-fg);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }
  .feature-check input:indeterminate + .check-box {
    background: var(--accent-dim);
    border-color: var(--accent);
  }
  .feature-check input:indeterminate + .check-box::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 6px;
    width: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
  }
  .feature-check input:disabled + .check-box,
  .file-check input:disabled + .check-box { opacity: 0.3; cursor: not-allowed; }

  .feature-glyph {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--feature-accent) 16%, var(--bg-elevated));
    border: 1px solid color-mix(in srgb, var(--feature-accent) 28%, transparent);
    color: var(--feature-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .feature-body { min-width: 0; }
  .feature-head { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
  .feature-title { font-size: var(--fs-base); font-weight: 600; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .feature-blurb { font-size: var(--fs-xs); color: var(--text-muted); margin-top: var(--space-1); line-height: var(--lh-snug); }

  .feature-versions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .ver-pair { display: inline-flex; align-items: center; gap: 6px; font-family: var(--font-mono); font-size: 11.5px; color: var(--text-secondary); }
  .ver { font-variant-numeric: tabular-nums; }
  .ver.current.is-update { color: var(--update); font-weight: 600; }
  .ver-pair .arrow { color: var(--text-muted); }
  .target { color: var(--accent); font-weight: 500; }
  .muted { color: var(--text-muted); font-style: italic; }
  .target-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font: inherit;
    color: inherit;
    font-family: var(--font-mono);
  }
  .target-btn:hover { background: var(--bg-card-hover); border-color: var(--border); }
  .target-btn .chev { color: var(--text-muted); }
  .target-btn.small { font-size: 11px; padding: 1px 6px; }
  .catalog-aside {
    display: inline-flex;
    align-items: center;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font: inherit;
    color: inherit;
    font-family: var(--font-mono);
  }
  .catalog-aside:hover { background: var(--bg-card-hover); border-color: var(--border); }
  .catalog-aside.small { font-size: 11px; padding: 1px 6px; }
  .pin-chip { padding: 1px 6px; font-size: 9px; margin-left: 4px; }

  .files-toggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10.5px;
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-xs);
    font-family: var(--font-sans);
  }
  .files-toggle:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .files-toggle .chev { transition: transform 0.15s var(--ease); }
  .files-toggle .chev.open { transform: rotate(180deg); }
  .files-toggle.subtle { color: var(--text-placeholder); }
  .files-toggle.subtle:hover { color: var(--text-secondary); }

  .feature-eye {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    flex-shrink: 0;
  }
  .feature-eye:hover { background: var(--bg-elevated); color: var(--text-primary); }

  .files-list {
    grid-column: 1 / -1;
    list-style: none;
    margin: 12px 0 2px;
    padding: 8px 6px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
  }
  .files-list.flat {
    grid-column: auto;
    margin: 0;
    background: var(--bg-card);
    border-radius: var(--radius-md);
  }
  .file-row {
    position: relative;
    display: grid;
    grid-template-columns: 22px 1fr auto;
    gap: 10px;
    align-items: flex-start;
    padding: 9px 10px;
    border-top: 1px solid var(--border);
  }
  .file-row:first-child { border-top: none; }
  .file-row:hover { background: var(--bg-card-hover); }
  .file-row.disabled { opacity: 0.5; }
  .file-check { display: inline-flex; cursor: pointer; padding-top: 2px; }
  .file-check input { display: none; }
  .file-info { min-width: 0; }
  .file-top { display: flex; align-items: baseline; gap: 8px; }
  .file-name { font-size: 11.5px; font-weight: 500; color: var(--text-primary); }
  .file-tag {
    font-size: 9.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .file-versions { display: flex; align-items: center; gap: 5px; margin-top: 3px; font-size: 10.5px; color: var(--text-secondary); }
  .file-versions .arrow { color: var(--text-muted); }
  .file-path { font-size: 9.5px; color: var(--text-muted); opacity: 0.7; margin-top: 3px; }
  .file-status { padding-top: 2px; }
  .small-chip { padding: 1px 7px; font-size: 9.5px; letter-spacing: 0.04em; }

  .advanced-block {
    background: transparent;
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    transition: background var(--dur-fast) var(--ease);
  }
  .advanced-block.open { background: transparent; }
  .advanced-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 14px 4px;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .advanced-head:hover { background: var(--bg-card-hover); border-radius: var(--radius-md); }
  .advanced-titles { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .advanced-name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .advanced-dot { width: 8px; height: 8px; border-radius: 50%; box-shadow: 0 0 6px currentColor; }
  .advanced-sub { font-size: 10.5px; color: var(--text-muted); margin-top: 2px; line-height: 1.4; }
  .advanced-chevron {
    color: var(--text-muted);
    transition: transform 0.18s var(--ease);
    display: inline-flex;
  }
  .advanced-chevron.open { transform: rotate(180deg); color: var(--text-primary); }
  .count.chip { padding: 1px 7px; }
  .dlss-drawer-body { padding: 0 4px 12px; display: flex; flex-direction: column; gap: 10px; }

  .drawer-foot {
    position: sticky;
    bottom: 0;
    flex-shrink: 0;
    margin: 0;
    padding: var(--space-3) var(--space-4);
    background: var(--glass-2);
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    border-top: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
    z-index: 4;
  }
  .drawer-foot::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: calc(-1 * var(--space-4));
    height: var(--space-4);
    background: linear-gradient(to top, var(--bg-card), transparent);
    pointer-events: none;
  }
  .drawer-foot .foot-util {
    flex: 0 0 auto;
    width: 40px;
    height: 40px;
    padding: 0;
    justify-content: center;
  }
  .drawer-foot .foot-util svg { margin: 0; }
  .drawer-foot .foot-apply {
    flex: 1 1 auto;
    min-width: 150px;
    height: 40px;
    order: 9;
    justify-content: center;
  }
  .drawer-foot .foot-apply.is-ac-danger {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--danger) 55%, transparent);
  }
  .drawer-foot .foot-streamline {
    flex: 1 1 100%;
    height: 40px;
    order: 8;
    justify-content: center;
  }
  .ahead-chip { order: 7; padding: 4px var(--space-3); flex: 0 0 auto; }

  .ac-apply-risk {
    order: 6;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px var(--space-3);
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: var(--letter-wide);
    text-transform: uppercase;
    line-height: 1;
  }
  .ac-apply-risk svg { flex-shrink: 0; }
  .ac-apply-risk.is-warning {
    color: var(--warning);
    background: var(--warning-dim);
    border: 1px solid color-mix(in srgb, var(--warning) 45%, transparent);
  }
  .ac-apply-risk.is-danger {
    color: var(--danger);
    background: var(--danger-dim);
    border: 1px solid color-mix(in srgb, var(--danger) 50%, transparent);
  }

  .ac-apply-confirm {
    order: 5;
    flex: 1 1 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--danger-dim);
    border: 1px solid var(--danger);
    color: var(--danger);
    animation: ac-confirm-in var(--dur-fast) var(--ease);
  }
  @keyframes ac-confirm-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .ac-confirm-text {
    flex: 1 1 200px;
    min-width: 0;
    font-size: var(--fs-sm);
    line-height: var(--lh-snug);
    overflow-wrap: anywhere;
  }
  .ac-confirm-actions { display: flex; gap: var(--space-2); flex: 0 0 auto; }

  @media (prefers-reduced-motion: reduce) {
    .ac-apply-confirm { animation: none; }
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  @container drawer (max-width: 420px) {
    .summary-row { gap: var(--space-1); }
    .summary-stat { padding: var(--space-2) var(--space-1); }
    .feature-row {
      grid-template-columns: 22px 1fr auto;
      column-gap: var(--space-2);
    }
    .feature-glyph { display: none; }
    .drawer-foot .foot-apply { flex: 1 1 100%; order: 9; }
  }
</style>
