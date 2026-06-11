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
    outdatedDllItems,
    backups,
    loadBackups,
    type StatusFilter,
  } from "../lib/stores";
  import { addBlacklistEntry, removeBlacklistEntry, openPath } from "../lib/api";
  import type { DetectedGame, LibraryViewMode, LibraryDensity, LibrarySort } from "../lib/api";
  import {
    LIBRARY_VIEW_MODES,
    LIBRARY_DENSITIES,
    LIBRARY_SORT_LABELS,
    LIBRARY_VIEW_MODE_DEFAULT,
    LIBRARY_DENSITY_DEFAULT,
    LIBRARY_SORT_DEFAULT,
  } from "../lib/ux";
  import { launcherLabel, familyGroup, GROUP_VENDOR, type FamilyGroup } from "../lib/labels";
  import GameCard from "../components/GameCard.svelte";
  import GameListRow from "../components/GameListRow.svelte";
  import FilterMenu from "../components/FilterMenu.svelte";
  import ContextMenu, { type ContextMenuAction, type ContextMenuItem } from "../components/ContextMenu.svelte";
  import { dispatchApply, type ApplyTarget } from "../lib/applyController";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { TRAY_SHOW_PROGRESS_EVENT } from "../lib/api";
  import { t, locale, translate } from "../lib/i18n/index";
  import { get } from "svelte/store";

  // Derive from the centralized `outdatedDllItems()` (shared with the daemon and
  // the digest). Touch the source stores so Svelte tracks them, since the helper
  // reads via `get(...)` internally.
  let outdatedItems = $derived.by(() => {
    void $games;
    void $gameDlls;
    void $gameStatuses;
    void $relationContext;
    void $settings;
    void $hiddenIds;
    return outdatedDllItems();
  });
  let outdatedTotal = $derived(outdatedItems.length);
  let outdatedBreakdown = $derived.by(() => {
    const counts: Record<FamilyGroup, number> = { dlss: 0, fsr: 0, xess: 0, advanced: 0 };
    for (const it of outdatedItems) counts[familyGroup(it.record.family)]++;
    const order: FamilyGroup[] = ["dlss", "fsr", "xess", "advanced"];
    return order
      .filter((g) => counts[g] > 0)
      .map((g) => ({ group: g, vendor: GROUP_VENDOR[g], count: counts[g] }));
  });
  let upToDateCount = $derived(
    $games.filter((g) => !$hiddenIds.has(g.id) && $gameStatuses[g.id] === "up_to_date").length,
  );
  let protectedGameCount = $derived.by(() => {
    const ids = new Set<string>();
    for (const b of $backups) {
      if (!b.restored_at && b.game_id && b.backup_type !== "driver_package") ids.add(b.game_id);
    }
    return ids.size;
  });

  async function updateAllOutdated(): Promise<void> {
    const items = outdatedDllItems();
    if (items.length === 0) {
      showToast("info", translate(get(locale), "view.library.toast.allUpToDate"));
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
        showToast(
          "warning",
          translate(get(locale), "view.library.toast.rescanAfterApplyFailed", {
            id: gid,
            error: String(err),
          }),
        );
      }
    }
  }

  let unlistenTrayProgress: UnlistenFn | undefined;
  onMount(() => {
    if ($games.length === 0) {
      void scanGames();
    }
    void loadBackups();
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
    { id: "all", labelKey: "view.library.launcherFilter.all", brand: null },
    { id: "steam", labelKey: null, brand: "Steam" },
    { id: "epic", labelKey: null, brand: "Epic" },
    { id: "gog", labelKey: null, brand: "GOG" },
    { id: "ubisoft", labelKey: null, brand: "Ubisoft" },
    { id: "ea_desktop", labelKey: null, brand: "EA" },
    { id: "xbox", labelKey: null, brand: "Xbox" },
    { id: "battlenet", labelKey: null, brand: "Battle.net" },
    { id: "manual", labelKey: "view.library.launcherFilter.custom", brand: null },
  ] as const;

  const statusFilters: { id: StatusFilter; labelKey: string }[] = [
    { id: "all", labelKey: "view.library.statusFilter.all" },
    { id: "outdated", labelKey: "status.outdated" },
    { id: "up_to_date", labelKey: "status.up_to_date" },
    { id: "no_dlls", labelKey: "status.no_dlls" },
    { id: "scan_failed", labelKey: "status.scan_failed" },
    { id: "hidden", labelKey: "view.library.statusFilter.hidden" },
  ];

  let hiddenCount = $derived($hiddenIds.size);

  let launcherOptions = $derived.by(() =>
    launcherFilters
      .map((f) => ({
        id: f.id,
        label: f.brand ?? translate($locale, f.labelKey),
        count: f.id === "all" ? $games.length : $games.filter((g) => g.launcher === f.id).length,
      }))
      .filter((o) => o.id === "all" || availableLaunchers.has(o.id) || o.count > 0),
  );

  let statusOptions = $derived.by(() =>
    statusFilters
      .filter((f) => f.id !== "hidden" || hiddenCount > 0)
      .map((f) => ({
        id: f.id,
        label: translate($locale, f.labelKey),
        count: f.id === "hidden" ? hiddenCount : undefined,
        tone: f.id === "hidden" ? ("danger" as const) : null,
      })),
  );

  function onCardClick(game: DetectedGame): void {
    drawerGameId.set(game.id);
  }
  function onApply(game: DetectedGame): void {
    drawerGameId.set(game.id);
  }
  async function onOpenFolder(game: DetectedGame): Promise<void> {
    try {
      await openPath(game.install_dir);
    } catch (err: unknown) {
      showToast(
        "danger",
        translate(get(locale), "view.library.toast.openFolderFailed", { error: String(err) }),
      );
    }
  }
  async function onHideToggle(game: DetectedGame): Promise<void> {
    const wasHidden = $hiddenIds.has(game.id);
    try {
      const next = wasHidden
        ? await removeBlacklistEntry(game.id)
        : await addBlacklistEntry(game.id);
      if ($settings) settings.set({ ...$settings, blacklist: next });
      showToast(
        wasHidden ? "success" : "info",
        translate(
          get(locale),
          wasHidden ? "view.library.toast.gameRestored" : "view.library.toast.gameHidden",
          { name: game.name },
        ),
      );
    } catch (err: unknown) {
      showToast(
        "danger",
        translate(
          get(locale),
          wasHidden ? "view.library.toast.restoreFailed" : "view.library.toast.hideFailed",
          { error: String(err) },
        ),
      );
    }
  }

  let contextMenu = $state<{ game: DetectedGame; x: number; y: number } | null>(null);
  let contextMenuItems = $derived.by<ContextMenuItem[]>(() => {
    if (!contextMenu) return [];
    const isHidden = $hiddenIds.has(contextMenu.game.id);
    const tr = (key: string): string => translate(get(locale), key);
    return [
      { action: "open_folder", label: tr("view.library.menu.openFolder") },
      { action: "scan", label: tr("view.library.menu.scan") },
      { action: "pin", label: tr("view.library.menu.pin") },
      { action: "hide", label: tr(isHidden ? "view.library.menu.unhide" : "view.library.menu.hide") },
    ];
  });

  function openContextMenu(game: DetectedGame, e: MouseEvent): void {
    contextMenu = { game, x: e.clientX, y: e.clientY };
  }

  async function onContextSelect(action: ContextMenuAction): Promise<void> {
    const game = contextMenu?.game;
    if (!game) return;
    switch (action) {
      case "open_folder":
        await onOpenFolder(game);
        break;
      case "scan":
        await rescanGame(game.id);
        showToast(
          "info",
          translate(get(locale), "view.library.toast.rescanning", { name: game.name }),
        );
        break;
      case "pin":
        drawerGameId.set(game.id);
        break;
      case "hide":
        await onHideToggle(game);
        break;
    }
  }

  async function addCustomFolder(): Promise<void> {
    if (!$settings) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({ directory: true, multiple: false });
      if (typeof result !== "string" || !result) return;
      if ($settings.launcher_overrides.custom.includes(result)) {
        showToast("warning", translate(get(locale), "view.library.toast.folderAlreadyAdded"));
        return;
      }
      await persistSettings({
        ...$settings,
        launcher_overrides: {
          ...$settings.launcher_overrides,
          custom: [...$settings.launcher_overrides.custom, result],
        },
      });
      showToast("success", translate(get(locale), "view.library.toast.folderAdded", { path: result }));
      await scanGames();
    } catch (err: unknown) {
      showToast(
        "danger",
        translate(get(locale), "view.library.toast.folderPickerFailed", { error: String(err) }),
      );
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

  // Tidal-style sections: split the actionable list into "Needs update" vs the rest.
  let needsUpdate = $derived(sortedActionable.filter((g) => $gameStatuses[g.id] === "outdated"));
  let upToDate = $derived(sortedActionable.filter((g) => $gameStatuses[g.id] !== "outdated"));

  let noDllsRevealed = $state(false);
  function toggleNoDllsZone(): void { noDllsRevealed = !noDllsRevealed; }

  function reviewChanges(): void {
    launcherFilter.set("all");
    statusFilter.set("outdated");
    searchQuery.set("");
  }

  function revealHidden(): void {
    launcherFilter.set("all");
    statusFilter.set("hidden");
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
    <h1 class="view-title">{$t("view.library.title")}</h1>
    <p class="view-subtitle">
      {$t("view.library.subtitle", { detected: $games.length, shown: $filteredGames.length })}
      {#if hiddenCount > 0 && $statusFilter !== "hidden"}
        <button
          type="button"
          class="hidden-chip"
          onclick={revealHidden}
          title={$t("view.library.hiddenChipTitle")}
        >
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
          {$t("view.library.hiddenChip", { count: hiddenCount })}
        </button>
      {/if}
    </p>
  </div>
  <div class="header-actions">
    <button class="btn btn-ghost" onclick={addCustomFolder}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/></svg>
      {$t("view.library.addFolder")}
    </button>
    <button class="btn" disabled={$scanInProgress} onclick={() => scanGames()}>
      {#if $scanInProgress}
        <span class="spin"></span>
        {$t("view.library.scanning")}
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        {$t("view.library.rescan")}
      {/if}
    </button>
  </div>
</header>

{#if $games.length > 0}
  <div class="updates-hero-shell">
    <aside
      class="updates-hero"
      data-state={outdatedTotal > 0 ? "pending" : "allclear"}
      role="status"
      aria-label={$t("view.library.hero.aria")}
    >
      <div class="updates-hero-body">
        <div class="updates-hero-lead">
          <span class="display-num" data-tone={outdatedTotal > 0 ? "warning" : "success"}
            >{outdatedTotal}</span
          >
          <div class="updates-hero-meta">
            <p class="updates-hero-headline">
              {outdatedTotal > 0
                ? $t("view.library.hero.updatesReadyLabel", { count: outdatedTotal })
                : $t("view.library.hero.allClear")}
            </p>
            <p class="updates-hero-scope">
              {outdatedTotal > 0
                ? $t("view.library.hero.acrossGames", { count: outdatedGameCount })
                : $t("view.library.hero.allClearDetail")}{#if $manifestUpdatedAt}<span
                  class="updates-hero-stamp"
                  title={$t("view.library.hero.manifestTitle")}
                  >&ensp;·&ensp;{$t("view.library.hero.manifestStamp", { stamp: $manifestUpdatedAt })}</span
                >{/if}
            </p>
          </div>
        </div>
        {#if outdatedTotal > 0}
          <ul class="updates-hero-tags" role="list">
            {#each outdatedBreakdown as bucket (bucket.group)}
              <li class="updates-hero-tag" data-group={bucket.group}>
                {bucket.group === "advanced"
                  ? $t("feature.advanced.short")
                  : $t("group." + bucket.group + ".label")}<span class="updates-hero-tag-n"
                  >{bucket.count}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <div class="updates-hero-kpis">
        <div class="hero-kpi">
          <span class="hero-kpi-num">{$games.length}</span>
          <span class="hero-kpi-label">{$t("view.library.hero.kpiGames")}</span>
        </div>
        <div class="hero-kpi" data-tone="success">
          <span class="hero-kpi-num">{upToDateCount}</span>
          <span class="hero-kpi-label">{$t("view.library.hero.kpiUpToDate")}</span>
        </div>
        <div class="hero-kpi" data-tone="info" title={$t("view.library.hero.kpiProtectedTitle")}>
          <span class="hero-kpi-num">{protectedGameCount}</span>
          <span class="hero-kpi-label">{$t("view.library.hero.kpiProtected")}</span>
        </div>
      </div>
      {#if outdatedTotal > 0}
        <div class="updates-hero-actions">
          <button
            class="updates-hero-review"
            onclick={reviewChanges}
            title={$t("view.library.hero.reviewTitle")}>{$t("view.library.hero.review")}</button
          >
          <button
            class="updates-hero-apply"
            onclick={updateAllOutdated}
            title={$t("view.library.hero.applyAllTitle")}>{$t("view.library.hero.applyAll")}</button
          >
        </div>
      {/if}
    </aside>
  </div>
{/if}

<div class="filter-shell">
<div class="filter-toolbar glass-panel" role="toolbar" aria-label={$t("view.library.filter.launcher")}>
  <FilterMenu
    label={$t("view.library.filter.launcher")}
    options={launcherOptions}
    selectedId={$launcherFilter}
    onSelect={(id) => launcherFilter.set(id)}
  />

  <FilterMenu
    label={$t("view.library.filter.status")}
    options={statusOptions}
    selectedId={$statusFilter}
    onSelect={(id) => statusFilter.set(id as StatusFilter)}
  />

  <div class="filter-controls">
    <select class="sort-select" value={sortKey} onchange={(e) => void setSort((e.currentTarget as HTMLSelectElement).value as LibrarySort)} aria-label={$t("view.library.filter.sortAria")} title={$t("view.library.filter.sort")}>
      {#each LIBRARY_SORT_LABELS as opt (opt.id)}
        <option value={opt.id} title={opt.hint ? $t("librarySort." + opt.id + ".hint") : $t("librarySort." + opt.id + ".label")}>{$t("librarySort." + opt.id + ".label")}</option>
      {/each}
    </select>
    <div class="seg" aria-label={$t("view.library.filter.view")}>
      <button class="seg-btn" class:active={viewMode === "grid"} onclick={() => void setViewMode("grid")} aria-pressed={viewMode === "grid"} title={$t("view.library.view.gridTitle")} aria-label={$t("view.library.view.grid")}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
        <span class="seg-text">{$t("view.library.view.grid")}</span>
      </button>
      <button class="seg-btn" class:active={viewMode === "list"} onclick={() => void setViewMode("list")} aria-pressed={viewMode === "list"} title={$t("view.library.view.listTitle")} aria-label={$t("view.library.view.list")}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>
        <span class="seg-text">{$t("view.library.view.list")}</span>
      </button>
    </div>
    <div class="seg" aria-label={$t("view.library.filter.density")}>
      <button class="seg-btn" class:active={density === "compact"} onclick={() => void setDensity("compact")} aria-pressed={density === "compact"} title={$t("view.library.density.compactTitle")}>{$t("view.library.density.compact")}</button>
      <button class="seg-btn" class:active={density === "comfy"} onclick={() => void setDensity("comfy")} aria-pressed={density === "comfy"} title={$t("view.library.density.comfyTitle")}>{$t("view.library.density.comfy")}</button>
    </div>
  </div>
</div>
</div>

{#if $scanInProgress && $games.length === 0}
  <div class="grid">
    {#each Array(8) as _, i (i)}
      <div class="card-skel">
        <div class="skel-art skeleton"></div>
        <div class="skel-body">
          <div class="skel-line skel-line-lg skeleton"></div>
          <div class="skel-line skel-line-sm skeleton"></div>
        </div>
      </div>
    {/each}
  </div>
{:else if $games.length === 0}
  <div class="empty">
    <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
    <h3 class="empty-title">{$t("view.library.empty.noGames.title")}</h3>
    <p class="section-sub">{$t("view.library.empty.noGames.detail")}</p>
    <p class="section-sub">{$t("view.library.empty.noGames.customPrefix")} <span class="mono">C:\Games</span>.</p>
    <div class="empty-actions">
      <button class="btn btn-primary" disabled={$scanInProgress} onclick={() => scanGames()}>
        {#if $scanInProgress}<span class="spin"></span>{$t("view.library.scanning")}{:else}{$t("view.library.empty.noGames.rescanNow")}{/if}
      </button>
      <button class="btn btn-ghost" onclick={addCustomFolder}>{$t("view.library.empty.noGames.addCustomFolder")}</button>
    </div>
  </div>
{:else if $filteredGames.length === 0}
  <div class="empty">
    <h3 class="empty-title">{$t("view.library.empty.noMatch.title")}</h3>
    <p class="section-sub">{$t("view.library.empty.noMatch.detail")}</p>
    <button class="btn btn-accent" onclick={() => { searchQuery.set(""); launcherFilter.set("all"); statusFilter.set("all"); }}>{$t("view.library.empty.noMatch.reset")}</button>
  </div>
{:else}
  {#snippet gameSection(title: string, list: DetectedGame[], viewAll: StatusFilter)}
    {#if list.length > 0}
      <section class="lib-section">
        <div class="section-head">
          <span class="section-title">{title}</span>
          <span class="section-count">{list.length}</span>
          {#if $statusFilter === "all"}
            <button class="section-viewall" onclick={() => statusFilter.set(viewAll)}>{$t("view.library.section.viewAll")}</button>
          {/if}
        </div>
        {#if viewMode === "grid"}
          <div class="grid media-deck" data-density={density}>
            {#each list as g, i (g.id)}
              <div class="grid-cell media-card" style:--stagger="{Math.min(i, 20) * 24}ms">
                <GameCard
                  game={g}
                  hidden={$hiddenIds.has(g.id)}
                  {onApply}
                  {onOpenFolder}
                  onBlacklist={onHideToggle}
                  onClick={onCardClick}
                  onContextMenu={openContextMenu}
                />
              </div>
            {/each}
          </div>
        {:else}
          <div class="list">
            {#each list as g, i (g.id)}
              <div class="list-cell" style:--stagger="{Math.min(i, 20) * 12}ms">
                <GameListRow
                  game={g}
                  hidden={$hiddenIds.has(g.id)}
                  {onApply}
                  {onOpenFolder}
                  onBlacklist={onHideToggle}
                  onClick={onCardClick}
                  onContextMenu={openContextMenu}
                />
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/snippet}

  {@render gameSection($t("view.library.section.needsUpdate"), needsUpdate, "outdated")}
  {@render gameSection($t("view.library.section.upToDate"), upToDate, "up_to_date")}

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
        <span class="zone-title">{$t("view.library.noDllsZone.title")}</span>
        <span class="zone-count">{$libraryZones.noDlls.length}</span>
        <span class="zone-hint">{$t("view.library.noDllsZone.hint")}</span>
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
                onContextMenu={openContextMenu}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/if}

{#if contextMenu}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    items={contextMenuItems}
    onSelect={(a) => void onContextSelect(a)}
    onClose={() => (contextMenu = null)}
  />
{/if}

<style>
  .view-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-4);
    gap: var(--space-4);
    flex-wrap: wrap;
  }
  .view-header > div:first-child { flex: 1 1 240px; min-width: 0; }
  .header-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; flex-shrink: 0; }
  .view-subtitle { display: inline-flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); }
  .hidden-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 9px 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .hidden-chip:hover { color: var(--text-primary); background: var(--bg-card-hover); border-color: var(--border-strong); }
  .hidden-chip:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .hidden-chip svg { color: var(--text-muted); flex-shrink: 0; }
  .filter-shell {
    container-type: inline-size;
    position: sticky;
    top: var(--space-2);
    z-index: 4;
    margin-bottom: var(--space-4);
  }
  .filter-toolbar {
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-lg);
    min-width: 0;
  }
  .filter-controls {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-inline-start: auto;
    flex-shrink: 1;
    min-width: 0;
  }
  .filter-controls .seg { flex-shrink: 0; }
  .seg-text { display: none; }
  @container (max-width: 640px) {
    .filter-toolbar { flex-wrap: wrap; row-gap: var(--space-2); }
    .filter-controls { margin-inline-start: 0; width: 100%; justify-content: flex-start; }
  }
  @container (max-width: 460px) {
    .sort-select { min-width: 0; flex: 1 1 auto; }
  }

  .updates-hero-shell { container-type: inline-size; margin-bottom: var(--space-3); }
  .updates-hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    column-gap: 16px;
    padding: 16px 20px 16px 22px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background:
      linear-gradient(120deg, color-mix(in oklab, var(--hero-tint, var(--accent)) 10%, transparent), transparent 52%),
      var(--bg-card);
    overflow: hidden;
  }
  .updates-hero[data-state="pending"] { --hero-tint: var(--warning); }
  .updates-hero[data-state="allclear"] { --hero-tint: var(--success); }
  .updates-hero-body { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .updates-hero-lead { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .updates-hero-meta { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .updates-hero-headline {
    margin: 0;
    font-size: 13px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-primary);
    line-height: 1.2;
  }
  .updates-hero-scope {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    min-width: 0;
  }
  .updates-hero-stamp {
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
    white-space: nowrap;
  }
  .updates-hero-kpis {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-shrink: 0;
    padding-inline: 16px;
    border-inline-start: 1px solid var(--border);
  }
  .hero-kpi { display: flex; flex-direction: column; gap: 3px; min-width: 48px; }
  .hero-kpi-num {
    font-size: 20px;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }
  .hero-kpi[data-tone="success"] .hero-kpi-num { color: var(--success); }
  .hero-kpi[data-tone="info"] .hero-kpi-num { color: var(--info); }
  .hero-kpi-label {
    font-size: 9.5px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .updates-hero-tags { display: flex; flex-wrap: wrap; gap: 6px; margin: 0; padding: 0; list-style: none; }
  .updates-hero-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3.5px 7px 3.5px 10px;
    border-radius: var(--radius-full);
    font-size: 10.5px;
    font-weight: 800;
    letter-spacing: 0.02em;
    color: var(--vendor-chip-fg);
    white-space: nowrap;
  }
  .updates-hero-tag-n {
    font-variant-numeric: tabular-nums;
    font-weight: 800;
    min-width: 16px;
    padding: 0 4px;
    text-align: center;
    border-radius: var(--radius-full);
    background: color-mix(in oklab, var(--vendor-chip-fg) 18%, transparent);
  }
  .updates-hero-tag[data-group="dlss"]     { background: var(--vendor-nvidia-ink); }
  .updates-hero-tag[data-group="fsr"]      { background: var(--vendor-amd-ink); }
  .updates-hero-tag[data-group="xess"]     { background: var(--vendor-intel-ink); }
  .updates-hero-tag[data-group="advanced"] { background: var(--vendor-microsoft-ink); }
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
  .updates-hero :global(.display-num) { font-size: clamp(28px, 3vw, 36px); }
  @container (max-width: 760px) {
    .updates-hero { grid-template-columns: minmax(0, 1fr) auto; row-gap: 14px; }
    .updates-hero-actions { grid-column: 1 / -1; width: 100%; }
    .updates-hero-actions button { flex: 1; }
  }
  @container (max-width: 620px) {
    .updates-hero { grid-template-columns: 1fr; padding-bottom: 20px; }
    .updates-hero-kpis { border-inline-start: none; padding-inline: 0; justify-content: flex-start; }
  }

  .sort-select {
    height: 32px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: var(--fs-sm);
    font-family: inherit;
    cursor: pointer;
    min-width: 7.5rem;
    max-width: 100%;
    flex-shrink: 1;
  }
  .sort-select:hover { border-color: var(--border-hover); }
  .sort-select:focus-visible { outline: none; border-color: var(--accent); box-shadow: var(--shadow-ring); }
  .lib-section { margin-bottom: 30px; }
  .lib-section:last-of-type { margin-bottom: 8px; }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
    gap: 16px;
    padding-bottom: 32px;
  }
  .lib-section .grid { padding-bottom: 4px; }
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
  .skel-line-lg { width: 70%; }
  .skel-line-sm { width: 50%; height: 10px; }

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
</style>
