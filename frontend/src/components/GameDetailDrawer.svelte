<script lang="ts">
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { t, locale, translate } from "../lib/i18n/index";
  import { setActiveArt, clearActiveArt } from "../lib/artContext";
  import { coverAccent } from "../lib/coverAccent";
  import { EXTERNAL_URLS } from "../lib/ux";
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
    ensureSystemInfo,
    fsr4Capable,
  } from "../lib/stores";
  import { dllRelation, targetVersion, recordUpdatable, isStreamlinePlugin } from "../lib/relation";
  import { addBlacklistEntry, removeBlacklistEntry, findGameExecutable, detectAnticheat, saveSettings, openPath, readDlssOverrideConfig, type AppSettings, type DllRecord, type AntiCheatReport } from "../lib/api";
  import { hasAntiCheat, statusNote, warningMessage, severity, detectedNames } from "../lib/anticheat";
  import { dispatchApply, dispatchStreamlineSet, dispatchDllSet, type ApplyTarget } from "../lib/applyController";
  import {
    familyLabel,
    familyShort,
    familyCatalogKey,
    launcherLabel,
    recordFeature,
    featureTitle,
    featureIconId,
    featureVendor,
    FEATURE_ORDER,
    VENDOR_ACCENTS,
    DLL_SET_FAMILIES,
    DLL_SET_LABELS,
    FSR4_GATED_FAMILIES,
    filenameFromPath,
    type FeatureSlot,
    type DllSetKey,
  } from "../lib/labels";
  import ContextMenu, { type ContextMenuAction } from "./ContextMenu.svelte";
  import DrawerHero from "./DrawerHero.svelte";
  import DrawerFeatureList, { type DrawerFeatureBucket, type DrawerAdvancedRow } from "./DrawerFeatureList.svelte";
  import DrawerFooter, { type DrawerDllSet } from "./DrawerFooter.svelte";

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
      void detectManagedExternally();
      void ensureSystemInfo();
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

  let featureBuckets = $derived.by<DrawerFeatureBucket[]>(() => {
    const map = new Map<FeatureSlot, DllRecord[]>();
    for (const r of records) {
      const f = recordFeature(r);
      if (!map.has(f)) map.set(f, []);
      map.get(f)!.push(r);
    }
    const out: DrawerFeatureBucket[] = [];
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
      let tone: DrawerFeatureBucket["statusTone"] = "neutral";
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

  let advancedRows = $derived.by<DrawerAdvancedRow[]>(() => {
    const map = new Map<string, DllRecord[]>();
    for (const r of advancedRecords) {
      const key = r.family === "streamline" ? `streamline:${filenameFromPath(r.path).toLowerCase()}` : r.family;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(r);
    }
    const out: DrawerAdvancedRow[] = [];
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

  function setFileSelection(key: string, checked: boolean): void {
    selected = { ...selected, [key]: checked };
  }

  function toggleFeatureSelection(bucket: DrawerFeatureBucket, checked: boolean): void {
    const next = { ...selected };
    for (const r of bucket.records) {
      if (disabledFamilies.includes(r.family)) continue;
      const rel = relation(r);
      if (rel === "same" || rel === "no-target") continue;
      next[rowKey(r)] = checked;
    }
    selected = next;
  }

  function featureSelectionState(bucket: DrawerFeatureBucket): "all" | "some" | "none" {
    const eligible = bucket.records.filter((r) => !disabledFamilies.includes(r.family) && relation(r) !== "same" && relation(r) !== "no-target");
    if (eligible.length === 0) return "none";
    const sel = eligible.filter((r) => selected[rowKey(r)]).length;
    if (sel === 0) return "none";
    if (sel === eligible.length) return "all";
    return "some";
  }

  function toggleFeatureExpanded(feature: string): void {
    expandedFeatures = { ...expandedFeatures, [feature]: !expandedFeatures[feature] };
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

  function dllSetMembers(key: DllSetKey): DllRecord[] {
    return records.filter((r) => DLL_SET_FAMILIES[key].includes(r.family) && isOutdated(r));
  }

  function versionMajor(version: string | null): number {
    const major = Number.parseInt(version?.split(".")[0] ?? "", 10);
    return Number.isNaN(major) ? 0 : major;
  }

  let fsrSetMembers = $derived(dllSetMembers("fsr"));
  let xessSetMembers = $derived(dllSetMembers("xess"));
  let fsrSetNeedsFsr4 = $derived(
    fsrSetMembers.some(
      (r) => FSR4_GATED_FAMILIES.includes(r.family) && versionMajor(targetFor(r)) >= 4,
    ),
  );
  let fsrSetBlocked = $derived(fsrSetNeedsFsr4 && !$fsr4Capable);

  let dllSets = $derived.by<DrawerDllSet[]>(() => {
    const out: DrawerDllSet[] = [];
    if (fsrSetMembers.length >= 2) {
      out.push({
        key: "fsr",
        label: DLL_SET_LABELS.fsr,
        count: fsrSetMembers.length,
        target: targetFor(fsrSetMembers[0]),
        blocked: fsrSetBlocked,
      });
    }
    if (xessSetMembers.length >= 2) {
      out.push({
        key: "xess",
        label: DLL_SET_LABELS.xess,
        count: xessSetMembers.length,
        target: targetFor(xessSetMembers[0]),
        blocked: false,
      });
    }
    return out;
  });

  async function applyDllSetAction(key: DllSetKey): Promise<void> {
    if (!game) return;
    const members = dllSetMembers(key);
    if (members.length === 0) return;
    const game_label = `${launcherLabel(game.launcher)} - ${game.name}`;
    const targets: ApplyTarget[] = [];
    for (const r of members) {
      const tgt = targetFor(r);
      if (!tgt) continue;
      targets.push({ game_id: game.id, game_label, record: r, target_version: tgt });
    }
    await dispatchDllSet(targets, DLL_SET_LABELS[key], { showModal: onApplyStart });
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

  let managedExternally = $state(false);
  async function detectManagedExternally(): Promise<void> {
    const id = gameId;
    managedExternally = false;
    await resolveExe();
    if (gameId !== id || !gameExe) return;
    try {
      const readback = await readDlssOverrideConfig({ scope: "per_game", executable_path: gameExe });
      if (gameId !== id) return;
      managedExternally =
        readback.source === "per_game" &&
        readback.active_count > 0 &&
        !$settings?.game_preferences[id];
    } catch {
      if (gameId === id) managedExternally = false;
    }
  }

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

  function openRowMenuAt(primaryKey: string, x: number, y: number): void {
    rowMenu = { x, y, primaryKey };
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
      await openPath(game.install_dir);
    } catch (err: unknown) {
      showToast("danger", translate(get(locale), "component.gameDrawer.toast.openFolderFailed", { error: String(err) }));
    }
  }

  async function openAnticheatLink(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(acLearnUrl);
    } catch (err) {
      showToast("warning", translate(get(locale), "component.gameDrawer.toast.openLinkFailed", { error: String(err) }));
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
  let acWarningMessage = $derived(acReport ? warningMessage(acReport) : "");

  let busy = $derived(loading || rescanning);
</script>

<svelte:window onkeydown={(e) => { if (game && e.key === "Escape") onClose(); }} />

{#if game}
  <div class="detail-view" aria-label={game.name}>
    <DrawerHero
      {game}
      {coverAccentColor}
      {loading}
      {rescanning}
      {scanError}
      recordCount={records.length}
      {outdatedCount}
      {aheadCount}
      {acActive}
      {acSeverity}
      {acStatus}
      {acWarningMessage}
      {dlssEnabler}
      {managedExternally}
      {onClose}
      onLearnMore={() => void openAnticheatLink()}
    />

    <div class="drawer-body">
      {#if busy}
        <div class="loading-state scanning" role="status" aria-live="polite">
          <div class="scan-head">
            <span class="spinner spin"></span>
            <span class="scan-label">{rescanning ? $t("component.gameDrawer.loading.rescanning") : $t("component.gameDrawer.loading.scanning")} — {game.install_dir.split(/[\\/]/).pop()}</span>
          </div>
          <div class="scan-skeleton" aria-hidden="true">
            {#each [70, 56, 64, 48] as w, i (i)}
              <div class="scan-skel-row">
                <div class="scan-skel-icon skeleton"></div>
                <div class="scan-skel-lines">
                  <div class="scan-skel-line skeleton" style:width="{w}%"></div>
                  <div class="scan-skel-line sm skeleton"></div>
                </div>
                <div class="scan-skel-pill skeleton"></div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        {#if scanError}
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
        {/if}
        <DrawerFeatureList
          hasRecords={!scanError && records.length > 0}
          recordCount={records.length}
          {outdatedCount}
          {selectedCount}
          {featureBuckets}
          {advancedRows}
          {selected}
          {disabledFamilies}
          {pinnedVersions}
          {expandedFeatures}
          {advancedExpanded}
          {pickerOpenFor}
          {dlssExpanded}
          dlssExe={gameExe}
          dlssExeResolving={exeResolving}
          dlssDriverPacked={nvidiaPacked}
          {rowKey}
          {relation}
          {targetFor}
          {latestFor}
          {featureSelectionState}
          onSelectAllOutdated={selectAllOutdated}
          onClearSelection={clearSelection}
          onToggleFeatureSelection={toggleFeatureSelection}
          onToggleFileSelection={setFileSelection}
          onToggleFeatureDisabled={(recs) => void toggleFeatureDisabled(recs)}
          onSetPin={(key, version) => void setPin(key, version)}
          onSetPickerOpen={(key) => (pickerOpenFor = key)}
          onToggleFeatureExpanded={toggleFeatureExpanded}
          onToggleAdvanced={() => (advancedExpanded = !advancedExpanded)}
          onToggleDlss={toggleDlss}
          onRowContextMenu={openRowMenu}
          onRowMenuAnchor={openRowMenuAt}
        />
      {/if}
    </div>

    <DrawerFooter
      {selectedCount}
      {aheadCount}
      {isHidden}
      {busy}
      streamlineSetCount={streamlineSetMembers.length}
      {streamlineSetTarget}
      {dllSets}
      {acActive}
      {acSeverity}
      {acNames}
      {acConfirming}
      onOpenFolder={() => void openFolder()}
      onRescan={() => void doRescan()}
      onToggleHidden={() => void toggleHidden()}
      onApplyStreamlineSet={() => void applyStreamlineSetAction()}
      onApplyDllSet={(key) => void applyDllSetAction(key)}
      onRequestApply={requestApply}
      onCancelApplyConfirm={cancelApplyConfirm}
    />
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
    --art-chrome-fg: #fff;
    --art-chrome-fg-dim: rgba(255, 255, 255, 0.88);
    --art-chrome-scrim: rgba(0, 0, 0, 0.55);
    --art-chrome-scrim-strong: rgba(0, 0, 0, 0.72);
    --art-chrome-border: rgba(255, 255, 255, 0.3);
    --art-chrome-border-strong: rgba(255, 255, 255, 0.5);
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

  .drawer-body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--space-4) var(--space-4) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: 48px 20px 36px;
    text-align: center;
    color: var(--text-muted);
  }
  .loading-state.scanning {
    align-items: stretch;
    text-align: left;
    padding: var(--space-2) 0 var(--space-3);
    gap: var(--space-4);
  }
  .scan-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    font-weight: 500;
  }
  .scan-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .scan-skeleton { display: flex; flex-direction: column; gap: var(--space-2); }
  .scan-skel-row {
    display: grid;
    grid-template-columns: 34px 1fr 56px;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
  }
  .scan-skel-icon { width: 34px; height: 34px; border-radius: var(--radius-md); }
  .scan-skel-lines { display: flex; flex-direction: column; gap: 7px; min-width: 0; }
  .scan-skel-line { height: 11px; border-radius: var(--radius-full); }
  .scan-skel-line.sm { height: 8px; width: 34%; }
  .scan-skel-pill { height: 18px; border-radius: var(--radius-full); }
  .empty-state h3 { color: var(--text-primary); font-size: var(--fs-lg); font-weight: 600; margin-top: 6px; }
  .empty-state p { font-size: var(--fs-sm); max-width: 360px; line-height: 1.55; }
  .empty-state svg { opacity: 0.6; }
  .empty-state .btn { margin-top: var(--space-3); }
  .empty-state .empty-actions { display: flex; gap: var(--space-2); margin-top: var(--space-3); }
  .empty-state.error-state svg { color: var(--danger); opacity: 0.85; }
  .empty-state.error-state h3 { color: var(--danger); }
  .empty-state .error-msg { color: var(--danger); background: var(--danger-dim); padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); font-size: var(--fs-xs); max-width: 100%; overflow-wrap: anywhere; }
  .empty-state .error-hint { color: var(--text-secondary); }

  .spinner {
    width: 14px;
    height: 14px;
  }
</style>
