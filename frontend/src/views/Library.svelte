<script lang="ts">
  import { onMount } from "svelte";
  import {
    games,
    filteredGames,
    libraryZones,
    scanInProgress,
    scanGames,
    searchQuery,
    launcherFilter,
    statusFilter,
    showToast,
    drawerGameId,
    settings,
    persistSettings,
    gameDlls,
    gameStatuses,
    relationContext,
    activeApplies,
    applyModalOpen,
    hiddenIds,
    rescanGame,
    manifestUpdatedAt,
    requestApplyAllOutdated,
    type StatusFilter,
  } from "../lib/stores";
  import { dllRelation, targetVersion } from "../lib/relation";
  import { addBlacklistEntry, removeBlacklistEntry, type DllRecord } from "../lib/api";
  import type { DetectedGame, LibraryViewMode, LibraryDensity, LibrarySort } from "../lib/api";
  import {
    LIBRARY_VIEW_MODES,
    LIBRARY_DENSITIES,
    LIBRARY_SORT_LABELS,
    LIBRARY_VIEW_MODE_DEFAULT,
    LIBRARY_DENSITY_DEFAULT,
    LIBRARY_SORT_DEFAULT,
  } from "../lib/ux";
  import { STATUS_LABELS, launcherLabel, familyGroup, GROUP_LABELS, type FamilyGroup } from "../lib/labels";
  import GameCard from "../components/GameCard.svelte";
  import GameListRow from "../components/GameListRow.svelte";
  import GameDetailDrawer from "../components/GameDetailDrawer.svelte";
  import { dispatchApply, type ApplyTarget } from "../lib/applyController";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { TRAY_SHOW_PROGRESS_EVENT } from "../lib/api";

  function outdatedDllsAcrossLibrary(): { game: DetectedGame; record: DllRecord; target: string }[] {
    const out: { game: DetectedGame; record: DllRecord; target: string }[] = [];
    for (const game of $games) {
      if ($hiddenIds.has(game.id)) continue;
      if ($gameStatuses[game.id] !== "outdated") continue;
      const records = $gameDlls[game.id] ?? [];
      const disabled = $settings?.game_preferences[game.id]?.disabled_families ?? [];
      const pinned = $settings?.game_preferences[game.id]?.pinned_versions ?? {};
      for (const r of records) {
        if (disabled.includes(r.family)) continue;
        const pin = pinned[`${r.family}|${r.path}`] ?? null;
        if (dllRelation(r, $relationContext, pin) !== "outdated") continue;
        const target = targetVersion(r, $relationContext, pin);
        if (!target) continue;
        out.push({ game, record: r, target });
      }
    }
    return out;
  }

  let outdatedItems = $derived(outdatedDllsAcrossLibrary());
  let outdatedTotal = $derived(outdatedItems.length);
  let outdatedBreakdown = $derived.by(() => {
    const counts: Record<FamilyGroup, number> = { dlss: 0, fsr: 0, xess: 0, advanced: 0 };
    for (const it of outdatedItems) counts[familyGroup(it.record.family)]++;
    const order: FamilyGroup[] = ["dlss", "fsr", "xess", "advanced"];
    return order.filter((g) => counts[g] > 0).map((g) => ({ group: g, label: GROUP_LABELS[g], count: counts[g] }));
  });

  async function updateAllOutdated(): Promise<void> {
    const items = outdatedDllsAcrossLibrary();
    if (items.length === 0) {
      showToast("info", "Everything is already up to date");
      return;
    }
    const targets: ApplyTarget[] = items.map((it) => ({
      game_id: it.game.id,
      game_label: `${launcherLabel(it.game.launcher)} - ${it.game.name}`,
      record: it.record,
      target_version: it.target,
    }));
    await dispatchApply(targets, { showModal: () => applyModalOpen.set(true) });
    const uniqueGames = new Set(items.map((i) => i.game.id));
    for (const gid of uniqueGames) {
      try {
        await rescanGame(gid);
      } catch (err: unknown) {
        showToast("warning", `Rescan after apply failed for ${gid}: ${String(err)}`);
      }
    }
  }

  let unlistenTrayProgress: UnlistenFn | undefined;
  onMount(() => {
    if ($games.length === 0) {
      void scanGames();
    }
    void listen<void>(TRAY_SHOW_PROGRESS_EVENT, () => {
      if (Object.keys($activeApplies).length > 0) {
        applyModalOpen.set(true);
      }
    }).then((un) => {
      unlistenTrayProgress = un;
    });
    return () => unlistenTrayProgress?.();
  });

  const launcherFilters = [
    { id: "all", label: "All" },
    { id: "steam", label: "Steam" },
    { id: "epic", label: "Epic" },
    { id: "gog", label: "GOG" },
    { id: "ubisoft", label: "Ubisoft" },
    { id: "ea_desktop", label: "EA" },
    { id: "xbox", label: "Xbox" },
    { id: "battlenet", label: "Battle.net" },
    { id: "manual", label: "Custom" },
  ] as const;

  const statusFilters: { id: StatusFilter; label: string }[] = [
    { id: "all", label: "All games" },
    { id: "outdated", label: STATUS_LABELS.outdated },
    { id: "up_to_date", label: STATUS_LABELS.up_to_date },
    { id: "no_dlls", label: STATUS_LABELS.no_dlls },
    { id: "scan_failed", label: STATUS_LABELS.scan_failed },
    { id: "hidden", label: "Hidden" },
  ];

  let hiddenCount = $derived($hiddenIds.size);

  function onCardClick(game: DetectedGame): void {
    drawerGameId.set(game.id);
  }
  function onApply(game: DetectedGame): void {
    drawerGameId.set(game.id);
  }
  async function onOpenFolder(game: DetectedGame): Promise<void> {
    try {
      const { openPath } = await import("../lib/api");
      await openPath(game.install_dir);
    } catch (err: unknown) {
      showToast("danger", `Open folder failed: ${String(err)}`);
    }
  }
  async function onHideToggle(game: DetectedGame): Promise<void> {
    const wasHidden = $hiddenIds.has(game.id);
    try {
      const next = wasHidden
        ? await removeBlacklistEntry(game.id)
        : await addBlacklistEntry(game.id);
      if ($settings) settings.set({ ...$settings, blacklist: next });
      showToast(wasHidden ? "success" : "info", `${game.name} ${wasHidden ? "restored" : "hidden"}`);
    } catch (err: unknown) {
      showToast("danger", `${wasHidden ? "Restore" : "Hide"} failed: ${String(err)}`);
    }
  }

  async function addCustomFolder(): Promise<void> {
    if (!$settings) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({ directory: true, multiple: false });
      if (typeof result !== "string" || !result) return;
      if ($settings.launcher_overrides.custom.includes(result)) {
        showToast("warning", "Folder already added");
        return;
      }
      await persistSettings({
        ...$settings,
        launcher_overrides: {
          ...$settings.launcher_overrides,
          custom: [...$settings.launcher_overrides.custom, result],
        },
      });
      showToast("success", `Added ${result}`);
      await scanGames();
    } catch (err: unknown) {
      showToast("danger", `Folder picker: ${String(err)}`);
    }
  }

  let availableLaunchers = $derived(new Set($games.map((g) => g.launcher)));

  let viewMode: LibraryViewMode = $derived(($settings?.ui_prefs.library_view_mode ?? LIBRARY_VIEW_MODE_DEFAULT) as LibraryViewMode);
  let density: LibraryDensity = $derived(($settings?.ui_prefs.library_density ?? LIBRARY_DENSITY_DEFAULT) as LibraryDensity);
  let sortKey: LibrarySort = $derived(($settings?.ui_prefs.library_sort ?? LIBRARY_SORT_DEFAULT) as LibrarySort);

  async function setViewMode(mode: LibraryViewMode): Promise<void> {
    if (!$settings || !LIBRARY_VIEW_MODES.includes(mode)) return;
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, library_view_mode: mode } });
  }
  async function setDensity(d: LibraryDensity): Promise<void> {
    if (!$settings || !LIBRARY_DENSITIES.includes(d)) return;
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, library_density: d } });
  }
  async function setSort(s: LibrarySort): Promise<void> {
    if (!$settings) return;
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, library_sort: s } });
  }

  const STATUS_SORT_RANK: Record<string, number> = {
    outdated: 0,
    scan_failed: 1,
    up_to_date: 2,
    no_dlls: 3,
    unknown: 4,
    scanning: 5,
  };

  function byOutdatedThenName(a: DetectedGame, b: DetectedGame): number {
    const ra = STATUS_SORT_RANK[$gameStatuses[a.id]] ?? 9;
    const rb = STATUS_SORT_RANK[$gameStatuses[b.id]] ?? 9;
    return ra - rb || a.name.localeCompare(b.name);
  }

  let sortedActionable = $derived.by(() => {
    const list = [...$libraryZones.actionable];
    switch (sortKey) {
      case "a_z":
        return list.sort((a, b) => a.name.localeCompare(b.name));
      case "z_a":
        return list.sort((a, b) => b.name.localeCompare(a.name));
      case "launcher":
        return list.sort((a, b) => a.launcher.localeCompare(b.launcher) || a.name.localeCompare(b.name));
      case "outdated_first":
      case "default":
      default:
        return list.sort(byOutdatedThenName);
    }
  });

  let outdatedGameCount = $derived.by(() => {
    const set = new Set<string>();
    for (const g of $games) {
      if (!$hiddenIds.has(g.id) && $gameStatuses[g.id] === "outdated") set.add(g.id);
    }
    return set.size;
  });

  let noDllsRevealed = $state(false);
  function toggleNoDllsZone(): void { noDllsRevealed = !noDllsRevealed; }

  function reviewChanges(): void {
    launcherFilter.set("all");
    statusFilter.set("outdated");
    searchQuery.set("");
  }

  let lastApplyAllSignal = $state(0);
  $effect(() => {
    const n = $requestApplyAllOutdated;
    if (n !== lastApplyAllSignal) {
      lastApplyAllSignal = n;
      if (n > 0) void updateAllOutdated();
    }
  });
</script>

<header class="view-header">
  <div>
    <h1 class="view-title">Library</h1>
    <p class="view-subtitle">{$games.length} games detected — {$filteredGames.length} shown</p>
  </div>
  <div class="header-actions">
    <button class="btn btn-ghost" onclick={addCustomFolder}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>
      Add folder
    </button>
    <button class="btn" disabled={$scanInProgress} onclick={() => scanGames()}>
      {#if $scanInProgress}
        <span class="spin"></span>
        Scanning
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        Rescan
      {/if}
    </button>
  </div>
</header>

{#if outdatedTotal > 0}
  <div class="updates-hero-shell">
    <aside class="updates-hero" role="status" aria-label="Pending updates">
      <span class="updates-hero-edge" aria-hidden="true"></span>
      <div class="updates-hero-body">
        <p class="updates-hero-headline">
          <strong>{outdatedTotal}</strong>
          update{outdatedTotal === 1 ? "" : "s"} ready
          <span class="updates-hero-scope"
            >across {outdatedGameCount} game{outdatedGameCount === 1 ? "" : "s"}</span
          >
        </p>
        <ul class="updates-hero-tags" role="list">
          {#each outdatedBreakdown as bucket (bucket.group)}
            <li class="updates-hero-tag" data-group={bucket.group}>
              {bucket.label}<span class="updates-hero-tag-n">{bucket.count}</span>
            </li>
          {/each}
        </ul>
      </div>
      <div class="updates-hero-actions">
        <button
          class="updates-hero-review"
          onclick={reviewChanges}
          title="Filter the library to just the games with pending updates">Review</button
        >
        <button
          class="updates-hero-apply"
          onclick={updateAllOutdated}
          title="Apply every detected update across the library">Apply all</button
        >
      </div>
      {#if $manifestUpdatedAt}
        <span class="updates-hero-stamp" title="Catalog manifest refresh time"
          >Manifest {$manifestUpdatedAt}</span
        >
      {/if}
    </aside>
  </div>
{/if}

<div class="filters-bar">
  <div class="filters-primary">
    <div class="filter-group">
      <span class="filter-group-label">Launcher</span>
      <div class="pills">
        {#each launcherFilters as f}
          {@const total = f.id === "all" ? $games.length : $games.filter((g) => g.launcher === f.id).length}
          {#if f.id === "all" || availableLaunchers.has(f.id) || total > 0}
            <button
              class="pill"
              class:active={$launcherFilter === f.id}
              onclick={() => launcherFilter.set(f.id)}
            >
              {f.label}
              <span class="pill-count">{total}</span>
            </button>
          {/if}
        {/each}
      </div>
    </div>
    <span class="filter-divider" aria-hidden="true"></span>
    <div class="filter-group">
      <span class="filter-group-label">Status</span>
      <div class="pills">
        {#each statusFilters as f}
          {#if f.id !== "hidden" || hiddenCount > 0}
            <button
              class="pill"
              class:active={$statusFilter === f.id}
              class:is-hidden={f.id === "hidden"}
              onclick={() => statusFilter.set(f.id)}
            >
              {f.label}
              {#if f.id === "hidden"}
                <span class="pill-count">{hiddenCount}</span>
              {/if}
            </button>
          {/if}
        {/each}
      </div>
    </div>
  </div>

  <div class="filters-secondary">
    <div class="filter-group">
      <span class="filter-group-label">Sort</span>
      <select class="sort-select" value={sortKey} onchange={(e) => void setSort((e.currentTarget as HTMLSelectElement).value as LibrarySort)} aria-label="Sort library">
        {#each LIBRARY_SORT_LABELS as opt (opt.id)}
          <option value={opt.id} title={opt.hint ?? opt.label}>{opt.label}</option>
        {/each}
      </select>
    </div>
    <div class="filter-group">
      <span class="filter-group-label">View</span>
      <div class="seg">
        <button class="seg-btn" class:active={viewMode === "grid"} onclick={() => void setViewMode("grid")} aria-pressed={viewMode === "grid"} title="Grid view">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
          Grid
        </button>
        <button class="seg-btn" class:active={viewMode === "list"} onclick={() => void setViewMode("list")} aria-pressed={viewMode === "list"} title="List view">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>
          List
        </button>
      </div>
    </div>
    <div class="filter-group">
      <span class="filter-group-label">Density</span>
      <div class="seg">
        <button class="seg-btn" class:active={density === "compact"} onclick={() => void setDensity("compact")} aria-pressed={density === "compact"} title="Compact density">Compact</button>
        <button class="seg-btn" class:active={density === "comfy"} onclick={() => void setDensity("comfy")} aria-pressed={density === "comfy"} title="Comfy density">Comfy</button>
      </div>
    </div>
  </div>
</div>

{#if $scanInProgress && $games.length === 0}
  <div class="grid">
    {#each Array(8) as _}
      <div class="card-skel">
        <div class="skel-art skeleton"></div>
        <div class="skel-body">
          <div class="skel-line skeleton" style="width: 70%"></div>
          <div class="skel-line skeleton" style="width: 50%; height: 10px;"></div>
        </div>
      </div>
    {/each}
  </div>
{:else if $games.length === 0}
  <div class="empty">
    <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
    <h3 class="empty-title">No games detected</h3>
    <p class="section-sub">Run a rescan to detect installed games from Steam, Epic, GOG, Ubisoft, EA, Xbox, and Battle.net.</p>
    <p class="section-sub">Or add a custom folder where you keep your games — for example <span class="mono">C:\Games</span>.</p>
    <div class="empty-actions">
      <button class="btn btn-primary" disabled={$scanInProgress} onclick={() => scanGames()}>
        {#if $scanInProgress}<span class="spin"></span>Scanning{:else}Rescan now{/if}
      </button>
      <button class="btn btn-ghost" onclick={addCustomFolder}>Add custom folder</button>
    </div>
  </div>
{:else if $filteredGames.length === 0}
  <div class="empty">
    <h3 class="empty-title">No games match your filters</h3>
    <p class="section-sub">Clear the search or pick a different filter.</p>
    <button class="btn btn-accent" onclick={() => { searchQuery.set(""); launcherFilter.set("all"); statusFilter.set("all"); }}>Reset filters</button>
  </div>
{:else}
  {#if sortedActionable.length > 0}
    {#if viewMode === "grid"}
      <div class="grid" data-density={density}>
        {#each sortedActionable as g, i (g.id)}
          <div class="grid-cell" style:--stagger="{Math.min(i, 20) * 24}ms">
            <GameCard
              game={g}
              hidden={$hiddenIds.has(g.id)}
              {onApply}
              {onOpenFolder}
              onBlacklist={onHideToggle}
              onClick={onCardClick}
            />
          </div>
        {/each}
      </div>
    {:else}
      <div class="list">
        {#each sortedActionable as g, i (g.id)}
          <div class="list-cell" style:--stagger="{Math.min(i, 20) * 12}ms">
            <GameListRow
              game={g}
              hidden={$hiddenIds.has(g.id)}
              {onApply}
              {onOpenFolder}
              onBlacklist={onHideToggle}
              onClick={onCardClick}
            />
          </div>
        {/each}
      </div>
    {/if}
  {/if}
  {#if $libraryZones.noDlls.length > 0}
    <div class="zone-no-dlls">
      <button
        class="zone-summary"
        class:is-open={noDllsRevealed}
        type="button"
        onclick={toggleNoDllsZone}
        aria-expanded={noDllsRevealed}
      >
        <svg class="zone-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
        <span class="zone-title">Games without supported technologies</span>
        <span class="zone-count">{$libraryZones.noDlls.length}</span>
        <span class="zone-hint">DLSS · FSR · XeSS not detected — click to expand</span>
      </button>
      {#if noDllsRevealed}
        <div class="grid grid-dimmed stagger" data-density={density}>
          {#each $libraryZones.noDlls as g (g.id)}
            <div>
              <GameCard
                game={g}
                hidden={$hiddenIds.has(g.id)}
                {onApply}
                {onOpenFolder}
                onBlacklist={onHideToggle}
                onClick={onCardClick}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/if}

{#if $drawerGameId}
  <GameDetailDrawer
    gameId={$drawerGameId}
    onClose={() => drawerGameId.set(null)}
    onApplyStart={() => applyModalOpen.set(true)}
  />
{/if}

<style>
  .view-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 18px;
    gap: 16px;
    flex-wrap: wrap;
  }
  .view-header > div:first-child { flex: 1 1 240px; min-width: 0; }
  .header-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; flex-shrink: 0; }
  .filters-bar {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-bottom: 22px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--border);
  }
  .filters-primary {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 14px 22px;
  }
  .filters-secondary {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 14px 20px;
  }
  .filter-divider {
    align-self: stretch;
    width: 1px;
    background: var(--border);
    margin: 2px 0;
  }
  @media (max-width: 720px) {
    .filter-divider { display: none; }
  }
  .filter-group { display: flex; flex-direction: column; gap: 7px; }
  .filter-group-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .pills { display: flex; flex-wrap: wrap; gap: 4px; }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 13px;
    border-radius: var(--radius-full);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 500;
    transition: background 0.15s var(--ease), border-color 0.15s var(--ease), color 0.15s var(--ease);
  }
  .pill:hover { background: var(--bg-elevated); border-color: var(--border-hover); color: var(--text-primary); }
  .pill.active { background: var(--accent-dim); border-color: var(--accent); color: var(--accent); }
  .pill.is-hidden { color: var(--text-muted); }
  .pill.is-hidden.active { background: rgba(239, 68, 68, 0.10); border-color: var(--danger); color: var(--danger); }
  .pill-count {
    font-size: 10px;
    font-weight: 600;
    background: var(--bg-elevated);
    color: var(--text-muted);
    padding: 1px 6px;
    border-radius: var(--radius-full);
  }
  .pill.active .pill-count { background: var(--accent-glow); color: var(--accent); }
  .pill.is-hidden.active .pill-count { background: rgba(239, 68, 68, 0.18); color: var(--danger); }

  .updates-hero-shell { container-type: inline-size; margin-bottom: 20px; }
  .updates-hero {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 16px 20px 16px 22px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background:
      linear-gradient(120deg, color-mix(in oklab, var(--accent) 7%, transparent), transparent 48%),
      var(--bg-card);
    overflow: hidden;
  }
  .updates-hero-edge {
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: linear-gradient(
      to bottom,
      var(--accent),
      color-mix(in oklab, var(--accent) 30%, transparent)
    );
  }
  .updates-hero-body { display: flex; flex-direction: column; gap: 9px; min-width: 0; }
  .updates-hero-headline {
    margin: 0;
    font-size: 14.5px;
    font-weight: 500;
    color: var(--text-secondary);
    letter-spacing: -0.005em;
  }
  .updates-hero-headline strong {
    font-size: 17px;
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    margin-right: 3px;
  }
  .updates-hero-scope { color: var(--text-muted); margin-left: 3px; }
  .updates-hero-tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 0; padding: 0; list-style: none; }
  .updates-hero-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px 3px 10px;
    border-radius: var(--radius-full);
    font-size: 11.5px;
    font-weight: 600;
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }
  .updates-hero-tag-n {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    min-width: 17px;
    padding: 0 5px;
    text-align: center;
    border-radius: var(--radius-full);
    background: color-mix(in oklab, currentColor 16%, transparent);
  }
  .updates-hero-tag[data-group="dlss"]     { color: var(--badge-green-fg); }
  .updates-hero-tag[data-group="fsr"]      { color: var(--badge-red-fg); }
  .updates-hero-tag[data-group="xess"]     { color: var(--badge-blue-fg); }
  .updates-hero-tag[data-group="advanced"] { color: var(--badge-purple-fg); }
  .updates-hero-actions { display: inline-flex; align-items: center; gap: 10px; flex-shrink: 0; }
  .updates-hero-review {
    height: 34px;
    padding: 0 14px;
    border-radius: var(--radius-md);
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 600;
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .updates-hero-review:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
    border-color: var(--border-strong);
  }
  .updates-hero-apply {
    height: 34px;
    padding: 0 18px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 12.5px;
    font-weight: 600;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
    transition: background var(--dur-fast) var(--ease);
  }
  .updates-hero-apply:hover { background: var(--accent-hover); }
  .updates-hero-stamp {
    position: absolute;
    right: 18px;
    bottom: 7px;
    font-size: 10px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
    opacity: 0.65;
  }
  @container (max-width: 620px) {
    .updates-hero { flex-direction: column; align-items: stretch; gap: 14px; padding-bottom: 20px; }
    .updates-hero-actions { width: 100%; }
    .updates-hero-actions button { flex: 1; }
    .updates-hero-stamp { display: none; }
  }

  .sort-select {
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius-md);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: var(--fs-sm);
    font-family: inherit;
    cursor: pointer;
    min-width: 160px;
  }
  .sort-select:hover { border-color: var(--border-hover); }
  .sort-select:focus-visible { outline: none; border-color: var(--accent); box-shadow: var(--shadow-ring); }
  .seg {
    display: inline-flex;
    height: 32px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }
  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 11px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 600;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .seg-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .seg-btn.active { background: var(--accent-dim); color: var(--accent); }
  .seg-btn:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
    gap: 16px;
    padding-bottom: 32px;
  }
  .grid[data-density="compact"] {
    grid-template-columns: repeat(auto-fill, minmax(184px, 1fr));
    gap: 10px;
  }
  .grid[data-density="compact"] :global(.body) { padding: 9px 11px 11px; gap: 6px; }
  .grid[data-density="compact"] :global(.game-name) { font-size: 12.5px; }
  .grid[data-density="compact"] :global(.feature-chip) { padding: 2px 6px 2px 5px; font-size: 10px; }
  .grid[data-density="compact"] :global(.launcher-text) { display: none; }
  .grid[data-density="compact"] :global(.launcher-chip) { padding: 4px; }
  @media (max-width: 720px) {
    .grid { grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; }
  }
  .grid-cell {
    opacity: 0;
    transform: translateY(8px);
    animation: cellIn var(--dur-slow) var(--ease-out) forwards;
    animation-delay: var(--stagger, 0ms);
  }
  @keyframes cellIn {
    to { opacity: 1; transform: translateY(0); }
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 32px;
  }
  .list-cell {
    opacity: 0;
    animation: cellIn var(--dur-normal) var(--ease-out) forwards;
    animation-delay: var(--stagger, 0ms);
  }

  .card-skel { background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
  .skel-art { aspect-ratio: var(--card-art-aspect); width: 100%; }
  .skel-body { padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 8px; }
  .skel-line { height: 14px; border-radius: var(--radius-sm); }

  .zone-no-dlls {
    margin-top: 28px;
    padding-top: 20px;
    border-top: 1px dashed var(--border);
  }
  .zone-summary {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .zone-summary:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .zone-summary:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .zone-chevron { color: var(--text-muted); transition: transform var(--dur-fast) var(--ease); flex-shrink: 0; }
  .zone-summary.is-open .zone-chevron { transform: rotate(90deg); color: var(--accent); }
  .zone-title { font-size: var(--fs-sm); font-weight: 600; letter-spacing: var(--letter-tight); }
  .zone-count {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .zone-hint { font-size: var(--fs-xs); color: var(--text-muted); margin-left: auto; }
  .grid-dimmed { margin-top: 14px; opacity: 0.72; }
  .grid-dimmed :global(.game-card) { background: var(--bg-card); }

  .empty { padding: 80px 0; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px; color: var(--text-muted); }
  .empty :global(svg) { margin-bottom: 8px; opacity: 0.5; }
  .empty-title { font-size: var(--fs-lg); font-weight: 600; color: var(--text-primary); margin-bottom: 4px; }
  .empty .section-sub { max-width: 480px; }
  .empty-actions { display: inline-flex; gap: 8px; margin-top: 16px; flex-wrap: wrap; justify-content: center; }

  .spin { width: 12px; height: 12px; border: 2px solid currentColor; border-top-color: transparent; border-radius: 50%; animation: spin 0.7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
