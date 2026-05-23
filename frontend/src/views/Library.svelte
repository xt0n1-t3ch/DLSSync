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
    hiddenIds,
    rescanGame,
    type ApplyTracker,
    type StatusFilter,
  } from "../lib/stores";
  import { dllRelation, targetVersion } from "../lib/relation";
  import { addBlacklistEntry, removeBlacklistEntry, applyUpdate, type DllRecord } from "../lib/api";
  import type { DetectedGame } from "../lib/api";
  import { STATUS_LABELS, familyCatalogKey, familyVendor, launcherLabel } from "../lib/labels";
  import GameCard from "../components/GameCard.svelte";
  import GameDetailDrawer from "../components/GameDetailDrawer.svelte";
  import ApplyProgressModal from "../components/ApplyProgressModal.svelte";

  let showApplyModal = $state(false);

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

  let outdatedTotal = $derived(outdatedDllsAcrossLibrary().length);

  async function updateAllOutdated(): Promise<void> {
    const items = outdatedDllsAcrossLibrary();
    if (items.length === 0) {
      showToast("info", "Everything is already up to date");
      return;
    }
    const trackers: Record<string, ApplyTracker> = {};
    for (const it of items) {
      const apply_id = crypto.randomUUID();
      trackers[apply_id] = {
        apply_id,
        game_id: it.game.id,
        game_label: `${launcherLabel(it.game.launcher)} - ${it.game.name}`,
        dll_path: it.record.path,
        family: it.record.family,
        target_version: it.target,
        stage: "download",
        failed_at_stage: null,
        message: "Queued",
        progress: null,
        error: null,
      };
    }
    activeApplies.set(trackers);
    showApplyModal = true;
    showToast("info", `Queued ${items.length} update${items.length === 1 ? "" : "s"} across ${new Set(items.map((i) => i.game.id)).size} game${items.length === 1 ? "" : "s"}`);
    for (const apply_id of Object.keys(trackers)) {
      const t = trackers[apply_id];
      try {
        await applyUpdate({
          apply_id,
          game_id: t.game_id,
          game_label: t.game_label,
          dll_path: t.dll_path,
          vendor: familyVendor(t.family as DllRecord["family"]),
          family: familyCatalogKey(t.family as DllRecord["family"]),
          target_version: t.target_version,
        });
      } catch (err: unknown) {
        const msg =
          err && typeof err === "object" && "message" in err
            ? String((err as { message: unknown }).message)
            : String(err);
        activeApplies.update((m) => {
          const existing = m[apply_id] ?? t;
          return {
            ...m,
            [apply_id]: {
              ...existing,
              stage: "failed",
              failed_at_stage: existing.failed_at_stage ?? existing.stage,
              error: msg,
              message: msg,
            },
          };
        });
      }
    }
    const uniqueGames = new Set(items.map((i) => i.game.id));
    for (const gid of uniqueGames) {
      try {
        await rescanGame(gid);
      } catch (err: unknown) {
        showToast("warning", `Rescan after apply failed for ${gid}: ${String(err)}`);
      }
    }
  }

  onMount(() => {
    if ($games.length === 0) {
      void scanGames();
    }
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
    {#if outdatedTotal > 0}
      <button class="btn btn-primary" onclick={updateAllOutdated} title="Apply all detected updates across every game">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        Update all ({outdatedTotal})
      </button>
    {/if}
  </div>
</header>

<div class="filters-row">
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
  {#if $libraryZones.actionable.length > 0}
    <div class="grid">
      {#each $libraryZones.actionable as g, i (g.id)}
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
  {/if}
  {#if $libraryZones.noDlls.length > 0}
    <details class="zone-no-dlls">
      <summary class="zone-summary">
        <svg class="zone-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
        <span class="zone-title">Games without supported technologies</span>
        <span class="zone-count">{$libraryZones.noDlls.length}</span>
        <span class="zone-hint">DLSS · FSR · XeSS not detected — click to expand</span>
      </summary>
      <div class="grid grid-dimmed">
        {#each $libraryZones.noDlls as g, i (g.id)}
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
    </details>
  {/if}
{/if}

{#if $drawerGameId}
  <GameDetailDrawer
    gameId={$drawerGameId}
    onClose={() => drawerGameId.set(null)}
    onApplyStart={() => (showApplyModal = true)}
  />
{/if}

{#if showApplyModal}
  <ApplyProgressModal onClose={() => (showApplyModal = false)} />
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
  .filters-row {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    margin-bottom: 22px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--border);
  }
  .filter-group { display: flex; flex-direction: column; gap: 6px; }
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
    padding: 5px 12px;
    border-radius: var(--radius-full);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
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

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
    gap: 16px;
    padding-bottom: 32px;
  }
  @media (max-width: 720px) {
    .grid { grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; }
  }
  .grid-cell {
    opacity: 0;
    transform: translateY(8px);
    animation: cellIn 0.36s var(--ease-out) forwards;
    animation-delay: var(--stagger, 0ms);
  }
  @keyframes cellIn {
    to { opacity: 1; transform: translateY(0); }
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
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    cursor: pointer;
    color: var(--text-secondary);
    list-style: none;
    user-select: none;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
  }
  .zone-summary::-webkit-details-marker { display: none; }
  .zone-summary:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .zone-chevron { color: var(--text-muted); transition: transform 0.2s var(--ease); flex-shrink: 0; }
  .zone-no-dlls[open] .zone-chevron { transform: rotate(90deg); color: var(--accent); }
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
