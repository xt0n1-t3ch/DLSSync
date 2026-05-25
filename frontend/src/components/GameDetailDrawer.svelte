<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { cubicOut, backIn } from "svelte/easing";
  import { EXTERNAL_URLS } from "../lib/ux";
  import {
    games,
    gameDlls,
    gameDllsLoading,
    gameDllErrors,
    catalogLatestByKey,
    relationContext,
    settings,
    persistSettings,
    showToast,
    rescanGame,
  } from "../lib/stores";
  import { dllRelation, targetVersion } from "../lib/relation";
  import { addBlacklistEntry, removeBlacklistEntry, type DllRecord } from "../lib/api";
  import { dispatchApply, type ApplyTarget } from "../lib/applyController";
  import {
    LAUNCHER_ACCENTS,
    familyLabel,
    familyShort,
    familyCatalogKey,
    launcherLabel,
    recordFeature,
    featureTitle,
    featureBlurb,
    featureIconId,
    featureVendor,
    FEATURE_ORDER,
    GROUP_LABELS,
    GROUP_SUB,
    GROUP_ACCENT,
    VENDOR_ACCENTS,
    filenameFromPath,
    type FeatureSlot,
  } from "../lib/labels";
  import VersionPickerPopover from "./VersionPickerPopover.svelte";
  import FeatureIcon from "./FeatureIcon.svelte";

  let { gameId, onClose, onApplyStart }: {
    gameId: string;
    onClose: () => void;
    onApplyStart: () => void;
  } = $props();

  let game = $derived($games.find((g) => g.id === gameId));
  let records: DllRecord[] = $derived($gameDlls[gameId] ?? []);
  let loading = $derived($gameDllsLoading[gameId] ?? false);
  let scanError = $derived($gameDllErrors[gameId] ?? null);
  let imgErrored = $state(false);
  let rescanning = $state(false);

  async function doRescan(): Promise<void> {
    if (rescanning) return;
    rescanning = true;
    try {
      await rescanGame(gameId);
      if (!scanError) showToast("success", "Rescan complete");
      else showToast("danger", `Rescan failed: ${scanError}`);
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
    }
  });

  function rowKey(r: DllRecord): string {
    return `${r.family}|${r.path}`;
  }

  function isOutdated(r: DllRecord): boolean {
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
    const families: Set<string> = new Set(recs.map((r) => r.family));
    const allDisabled = [...families].every((f) => disabledFamilies.includes(f));
    const prefs = { ...$settings.game_preferences };
    const cur = prefs[gameId] ?? { disabled_families: [], pinned_versions: {} };
    let next = [...cur.disabled_families];
    if (allDisabled) {
      next = next.filter((f) => !families.has(f));
    } else {
      for (const f of families) if (!next.includes(f)) next.push(f);
    }
    prefs[gameId] = { ...cur, disabled_families: next };
    await persistSettings({ ...$settings, game_preferences: prefs });
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
      const allUpToDate = recs.length > 0 && recs.some((r) => relation(r) === "same");
      const allDisabled = recs.every((r) => disabledFamilies.includes(r.family));
      let label = "No catalog";
      let tone: FeatureBucket["statusTone"] = "neutral";
      if (allDisabled) { label = "Disabled"; tone = "neutral"; }
      else if (anyOutdated) { label = recs.length > 1 ? "Updates ready" : "Update ready"; tone = "update"; }
      else if (anyAhead) { label = "Ahead of catalog"; tone = "info"; }
      else if (allUpToDate) { label = "Up to date"; tone = "success"; }
      out.push({
        feature: fid,
        records: recs,
        primary,
        title: featureTitle(fid),
        blurb: featureBlurb(fid),
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

  async function applySelected(): Promise<void> {
    if (!game) return;
    const items = selectedRecords();
    if (items.length === 0) {
      showToast("warning", "Nothing selected");
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
        `Rescan after apply failed: ${String(err)} — close and re-open the game to refresh`,
      );
    }
  }

  async function openFolder(): Promise<void> {
    if (!game) return;
    try {
      const { openPath } = await import("../lib/api");
      await openPath(game.install_dir);
    } catch (err: unknown) {
      showToast("danger", `Open folder: ${String(err)}`);
    }
  }

  let isHidden = $derived(($settings?.blacklist ?? []).includes(gameId));

  async function toggleHidden(): Promise<void> {
    if (!game) return;
    const wasHidden = isHidden;
    try {
      const next = wasHidden ? await removeBlacklistEntry(game.id) : await addBlacklistEntry(game.id);
      if ($settings) settings.set({ ...$settings, blacklist: next });
      showToast(wasHidden ? "success" : "info", `${game.name} ${wasHidden ? "restored" : "hidden"}`);
      if (!wasHidden) onClose();
    } catch (err: unknown) {
      showToast("danger", `${wasHidden ? "Restore" : "Hide"} failed: ${String(err)}`);
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

  let anticheatHint = $derived.by(() => {
    if (!game) return null;
    const d = game.install_dir.replace(/\\/g, "/").toLowerCase();
    if (d.includes("/easyanticheat") || d.includes("/eac")) return "Easy Anti-Cheat";
    if (d.includes("/battleye")) return "BattlEye";
    if (d.includes("/vac")) return "Valve Anti-Cheat";
    return null;
  });
</script>

<svelte:window onkeydown={(e) => { if (game && e.key === "Escape") onClose(); }} />

{#if game}
  <div class="drawer-scrim" role="presentation" onclick={onClose} transition:fade={{ duration: 180 }}></div>
  <aside class="drawer" in:fly={{ x: 480, duration: 220, easing: cubicOut }} out:fly={{ x: 480, duration: 280, easing: backIn }}>
    <header class="drawer-head" style:--launcher-accent={accent}>
      <div class="drawer-art">
        {#if game.image_url && !imgErrored}
          <img src={game.image_url} alt={game.name} onerror={() => (imgErrored = true)} />
        {:else}
          <div class="drawer-art-fallback">{game.name.slice(0, 1).toUpperCase()}</div>
        {/if}
        <div class="drawer-art-overlay"></div>
      </div>
      <button class="drawer-close" onclick={onClose} title="Close (Esc)" aria-label="Close">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
      <div class="drawer-meta">
        <span class="launcher-chip" style:background={accent}>{launcherLabel(game.launcher)}</span>
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
        <span>Scanning DLLs…</span>
      {:else if scanError}
        <span class="ribbon-dot"></span>
        <span>Scan failed: <span class="mono">{scanError}</span></span>
      {:else if records.length === 0}
        <span class="ribbon-dot"></span>
        <span>No supported DLLs detected</span>
      {:else if outdatedCount === 0}
        <span class="ribbon-dot"></span>
        <span>All up to date · {records.length} file{records.length === 1 ? "" : "s"}</span>
      {:else}
        <span class="ribbon-dot is-pulse"></span>
        <span>{outdatedCount} update{outdatedCount === 1 ? "" : "s"} ready{aheadCount > 0 ? ` · ${aheadCount} ahead of catalog` : ""}</span>
      {/if}
    </div>

    <div class="drawer-body">
      {#if anticheatHint}
        <div class="warning-banner" role="alert">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          <span><strong>{anticheatHint}</strong> detected in this install — patching DLLs may trigger a kick or ban. Verify with the developer before applying.</span>
          <button
            class="learn-more"
            title="Open the README anti-cheat FAQ on GitHub"
            onclick={async () => {
              try {
                const { open } = await import("@tauri-apps/plugin-shell");
                await open(EXTERNAL_URLS.anticheatFaq);
              } catch (err) { showToast("warning", `Open link failed: ${String(err)}`); }
            }}
          >Learn more</button>
        </div>
      {/if}

      {#if loading || rescanning}
        <div class="loading-state">
          <span class="spinner"></span>
          <span>{rescanning ? "Rescanning" : "Scanning DLLs"} — {game.install_dir.split(/[\\/]/).pop()}</span>
        </div>
      {:else if scanError}
        <div class="empty-state error-state">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
          <h3>Scan failed</h3>
          <p class="error-msg mono">{scanError}</p>
          <p class="error-hint">This is usually a transient I/O race during the initial concurrent scan. Click below to retry just this game.</p>
          <button class="btn btn-primary" onclick={doRescan}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
            Rescan this game
          </button>
        </div>
      {:else if records.length === 0}
        <div class="empty-state">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <h3>No DLSS, FSR or XeSS files found</h3>
          <p>We scanned this folder and didn't find any tracked upscaling DLLs. If the game ships them in a custom subfolder, try Rescan or open the folder to check.</p>
          <div class="empty-actions">
            <button class="btn btn-accent" onclick={doRescan}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
              Rescan
            </button>
            <button class="btn btn-ghost" onclick={openFolder}>Open folder</button>
          </div>
        </div>
      {:else}
        <div class="summary-row">
          <div class="summary-stat">
            <span class="stat-num">{records.length}</span>
            <span class="stat-label">Files</span>
          </div>
          <div class="summary-stat">
            <span class="stat-num" class:is-update={outdatedCount > 0}>{outdatedCount}</span>
            <span class="stat-label">Updates</span>
          </div>
          <div class="summary-stat">
            <span class="stat-num" class:is-accent={selectedCount > 0}>{selectedCount}</span>
            <span class="stat-label">Selected</span>
          </div>
        </div>

        {#if outdatedCount > 0}
          <div class="quick-actions">
            <button class="btn btn-sm btn-accent" onclick={selectAllOutdated}>
              Select all updates ({outdatedCount})
            </button>
            <button class="btn btn-sm btn-ghost" onclick={clearSelection} disabled={selectedCount === 0}>
              Clear selection
            </button>
          </div>
        {/if}

        {#if featureBuckets.length > 0}
          <ul class="feature-list">
            {#each featureBuckets as b (b.feature)}
              {@const selState = featureSelectionState(b)}
              {@const expanded = !!expandedFeatures[b.feature]}
              {@const primaryKey = rowKey(b.primary)}
              {@const primaryRel = relation(b.primary)}
              {@const primaryTarget = targetFor(b.primary)}
              {@const primaryLatest = latestFor(b.primary)}
              {@const primaryPinned = pinnedVersions[primaryKey]}
              {@const primaryAside = primaryRel === "ahead" || (primaryRel === "same" && primaryTarget != null && primaryTarget !== (b.primary.current_version ?? ""))}
              <li class="feature-row" class:is-update={b.anyOutdated && !b.allDisabled} class:disabled={b.allDisabled}>
                <label class="feature-check" title={selState === "all" ? "Deselect all in this feature" : "Select all updates in this feature"}>
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
                        <button class="ver catalog-aside" onclick={() => (pickerOpenFor = primaryKey)} title="Pick a different version">
                          <span class="muted">· catalog v{primaryTarget}</span>
                        </button>
                      {:else}
                        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                        <button class="ver target-btn" onclick={() => (pickerOpenFor = primaryKey)} title="Choose a version">
                          {#if primaryTarget}
                            <span class="target">v{primaryTarget}</span>
                            {#if primaryPinned && primaryPinned !== primaryLatest}
                              <span class="chip chip-update pin-chip" title="Pinned by you">Pinned</span>
                            {/if}
                          {:else}
                            <span class="muted">choose version</span>
                          {/if}
                          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                        </button>
                      {/if}
                    </span>
                    {#if b.records.length > 1}
                      <button class="files-toggle" onclick={() => (expandedFeatures = { ...expandedFeatures, [b.feature]: !expanded })} aria-expanded={expanded}>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                        {expanded ? "Hide files" : `Show files (${b.records.length})`}
                      </button>
                    {:else}
                      <button class="files-toggle subtle" onclick={() => (expandedFeatures = { ...expandedFeatures, [b.feature]: !expanded })} aria-expanded={expanded} title={filenameFromPath(b.primary.path)}>
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                        {expanded ? "Hide file" : "Show file"}
                      </button>
                    {/if}
                  </div>
                </div>
                <button class="feature-eye" onclick={() => void toggleFeatureDisabled(b.records)} title={b.allDisabled ? "Re-enable feature for this game" : "Disable feature for this game"} aria-label="Toggle feature">
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
                      {@const t = targetFor(r)}
                      {@const lat = latestFor(r)}
                      {@const rel = relation(r)}
                      {@const fd = disabledFamilies.includes(r.family)}
                      {@const pin = pinnedVersions[k]}
                      {@const fileAside = rel === "ahead" || (rel === "same" && t != null && t !== (r.current_version ?? ""))}
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
                              <button class="ver catalog-aside small" onclick={() => (pickerOpenFor = k)} title="Pick a different version">
                                <span class="muted mono">· catalog v{t}</span>
                              </button>
                            {:else}
                              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                              <button class="ver target-btn small" onclick={() => (pickerOpenFor = k)}>
                                {#if t}
                                  <span class="target mono">v{t}</span>
                                  {#if pin && pin !== lat}<span class="chip chip-update pin-chip">Pinned</span>{/if}
                                {:else}
                                  <span class="muted">choose version</span>
                                {/if}
                                <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                              </button>
                            {/if}
                          </div>
                          <div class="file-path mono truncate" title={r.path}>{r.path}</div>
                        </div>
                        <div class="file-status">
                          {#if rel === "outdated"}
                            <span class="chip chip-update small-chip">Update</span>
                          {:else if rel === "ahead"}
                            <span class="chip chip-info small-chip" title="Installed version is newer than what the catalog tracks">Ahead</span>
                          {:else if rel === "same"}
                            <span class="chip chip-success small-chip">Current</span>
                          {:else}
                            <span class="chip chip-neutral small-chip">No catalog</span>
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
                  {GROUP_LABELS.advanced}
                  <span class="chip chip-neutral small-chip count">{advancedRows.length}</span>
                </span>
                <span class="advanced-sub">{GROUP_SUB.advanced}</span>
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
                  {@const t = targetFor(r)}
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
                          {#if t}<span class="target mono">v{t}</span>{:else}<span class="muted">choose version</span>{/if}
                          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                        </button>
                      </div>
                    </div>
                    <div class="file-status">
                      {#if rel === "outdated"}
                        <span class="chip chip-update small-chip">Update</span>
                      {:else if rel === "ahead"}
                        <span class="chip chip-info small-chip" title="Installed version is newer than what the catalog tracks">Ahead</span>
                      {:else if rel === "same"}
                        <span class="chip chip-success small-chip">Current</span>
                      {:else}
                        <span class="chip chip-neutral small-chip">No catalog</span>
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
    </div>

    <footer class="drawer-foot">
      <button class="btn btn-ghost" onclick={openFolder} title="Open install folder">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
        Open folder
      </button>
      <button class="btn btn-ghost" onclick={doRescan} title="Rescan this game's install folder" disabled={loading || rescanning}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        Rescan
      </button>
      <button class="btn btn-ghost" onclick={toggleHidden}>{isHidden ? "Restore" : "Hide"}</button>
      <div class="foot-spacer"></div>
      {#if aheadCount > 0}
        <span class="chip chip-info ahead-chip">{aheadCount} ahead of catalog</span>
      {/if}
      <button
        class="btn btn-primary halo is-update"
        class:is-active={selectedCount > 0}
        disabled={selectedCount === 0}
        onclick={applySelected}
      >
        Apply selected ({selectedCount})
      </button>
    </footer>
  </aside>
{/if}

<style>
  .drawer-scrim { display: none; }
  @media (max-width: 1300px) {
    .drawer-scrim {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.5);
      backdrop-filter: blur(2px);
      -webkit-backdrop-filter: blur(2px);
      z-index: 150;
    }
  }
  .drawer {
    position: fixed;
    right: 0;
    top: 0;
    bottom: 0;
    width: var(--drawer-width);
    background: var(--bg-card);
    border-left: 1px solid var(--border-strong);
    box-shadow: -12px 0 40px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    z-index: 151;
    overflow: hidden;
  }
  .drawer-head { position: relative; }
  .drawer-art { width: 100%; aspect-ratio: 16 / 9; overflow: hidden; }
  .drawer-art img { width: 100%; height: 100%; object-fit: cover; }
  .drawer-art-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-art-fallback);
    color: var(--launcher-accent, var(--accent));
    font-size: 96px;
    font-weight: 700;
    opacity: 0.55;
  }
  .drawer-art-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(0,0,0,0.30) 0%, rgba(0,0,0,0) 35%, rgba(0,0,0,0) 55%, rgba(0,0,0,0.95) 100%);
    pointer-events: none;
  }
  .drawer-close {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(8px);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2;
  }
  .drawer-close:hover { background: rgba(0, 0, 0, 0.85); }
  .drawer-meta {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 18px 22px 20px;
    z-index: 1;
  }
  .launcher-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px 9px;
    border-radius: var(--radius-full);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
    color: #0a0d13;
    margin-bottom: 8px;
  }
  .drawer-title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: #fff;
    margin-bottom: 4px;
    text-shadow: 0 2px 8px rgba(0,0,0,0.8);
  }
  .drawer-path { font-size: 11px; color: rgba(255,255,255,0.78); }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 18px 22px 24px;
  }

  .warning-banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background: var(--warning-dim);
    border: 1px solid var(--warning);
    color: var(--warning);
    font-size: 11.5px;
    line-height: 1.45;
    margin-bottom: 16px;
  }
  .warning-banner svg { flex-shrink: 0; margin-top: 1px; }
  .learn-more {
    margin-left: auto;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.08);
    color: currentColor;
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    border: 1px solid currentColor;
    flex-shrink: 0;
    transition: background var(--dur-fast) var(--ease);
  }
  .learn-more:hover { background: rgba(255, 255, 255, 0.16); }
  .learn-more:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .status-ribbon {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    background: var(--bg-elevated);
    font-variant-numeric: tabular-nums;
  }
  .status-ribbon.is-update { color: var(--update); background: var(--update-dim); border-bottom-color: var(--update-glow); }
  .status-ribbon.is-success { color: var(--success); background: var(--success-dim); border-bottom-color: var(--success-glow); }
  .status-ribbon.is-danger { color: var(--danger); background: var(--danger-dim); border-bottom-color: var(--danger-glow); }
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
    display: flex;
    gap: 6px;
    padding: 10px 12px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    margin-bottom: 14px;
  }
  .summary-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4px;
  }
  .stat-num {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tighter);
    font-variant-numeric: tabular-nums;
  }
  .stat-num.is-update { color: var(--update); }
  .stat-num.is-accent { color: var(--accent); }
  .stat-label {
    font-size: 9.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    margin-top: 2px;
  }

  .quick-actions { display: flex; gap: 8px; margin-bottom: 14px; }

  .feature-list { list-style: none; padding: 0; margin: 0 0 16px; display: flex; flex-direction: column; gap: 8px; }
  .feature-row {
    position: relative;
    display: grid;
    grid-template-columns: 22px 36px 1fr auto;
    gap: 12px;
    align-items: flex-start;
    padding: 14px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition: border-color 0.15s var(--ease), background 0.15s var(--ease);
  }
  .feature-row.is-update { border-color: rgba(34, 211, 238, 0.30); }
  .feature-row.is-update:hover { background: var(--bg-card-hover); border-color: var(--accent-ring); box-shadow: 0 0 0 3px var(--accent-dim); }
  .feature-row:hover { background: var(--bg-card-hover); }
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
    background: color-mix(in srgb, var(--feature-accent) 14%, var(--bg-elevated));
    color: var(--feature-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .feature-body { min-width: 0; }
  .feature-head { display: flex; align-items: center; gap: 8px; }
  .feature-title { font-size: 13.5px; font-weight: 600; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .feature-blurb { font-size: 11px; color: var(--text-muted); margin-top: 2px; line-height: 1.45; }

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
    margin: 14px -4px 0;
    padding: 8px 6px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
  }
  .files-list.flat {
    grid-column: auto;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    margin: 0;
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
    margin-top: 8px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: background 0.15s var(--ease);
  }
  .advanced-block.open { background: var(--bg-elevated); }
  .advanced-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 12px 14px;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .advanced-head:hover { background: var(--bg-elevated); }
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

  .drawer-foot {
    padding: 12px 22px;
    border-top: 1px solid var(--border);
    background: var(--bg-input);
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    container-type: inline-size;
  }
  .foot-spacer { flex: 1 1 0; min-width: 0; }
  .ahead-chip { padding: 3px 8px; flex-shrink: 0; }

  @container (max-width: 460px) {
    .drawer-foot { padding: 12px 14px; gap: 6px; row-gap: 8px; }
    .drawer-foot .btn-primary {
      order: 99;
      width: 100%;
      justify-content: center;
    }
    .foot-spacer { display: none; }
    .ahead-chip { order: 98; flex: 1 0 100%; text-align: center; }
  }

  @container (max-width: 360px) {
    .drawer-foot .btn-ghost {
      padding-left: 8px;
      padding-right: 8px;
      font-size: 0;
    }
    .drawer-foot .btn-ghost svg { margin: 0; }
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
</style>
