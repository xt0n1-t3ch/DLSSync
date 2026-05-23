<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { backups, loadBackups, games, showToast } from "../lib/stores";
  import { restoreBackup, deleteBackup, openPath, type BackupEntry, type DetectedGame } from "../lib/api";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import {
    featureTitle,
    featureFromFamily,
    featureIconId,
    launcherAccent,
    launcherLabel,
    DEFAULT_VENDOR_ACCENT,
  } from "../lib/labels";
  import FeatureIcon from "./../components/FeatureIcon.svelte";

  onMount(() => {
    void loadBackups();
  });

  type GroupedBackup = {
    game_id: string;
    name: string;
    game: DetectedGame | null;
    entries: BackupEntry[];
    activeCount: number;
    restoredCount: number;
    latestAt: string;
    oldestAt: string;
    sizeBytes: number;
  };

  let query = $state("");
  let expanded = $state<Record<string, boolean>>({});
  let restoringId = $state<string | null>(null);
  let deletingId = $state<string | null>(null);
  let openingPath = $state<string | null>(null);
  let selectedIds = $state<Set<string>>(new Set());
  let bulkRunning = $state<"restore" | "delete" | null>(null);

  function toggleEntry(id: string): void {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function groupSelectionState(g: GroupedBackup): "none" | "some" | "all" {
    let on = 0;
    for (const e of g.entries) if (selectedIds.has(e.id)) on += 1;
    if (on === 0) return "none";
    if (on === g.entries.length) return "all";
    return "some";
  }

  function toggleGroupSelection(g: GroupedBackup, checked: boolean): void {
    const next = new Set(selectedIds);
    for (const e of g.entries) {
      if (checked) next.add(e.id);
      else next.delete(e.id);
    }
    selectedIds = next;
  }

  function clearSelection(): void {
    selectedIds = new Set();
  }

  let gameById = $derived.by<Map<string, DetectedGame>>(() => {
    const m = new Map<string, DetectedGame>();
    for (const g of $games) m.set(g.id, g);
    return m;
  });

  let grouped = $derived.by<GroupedBackup[]>(() => {
    const m = new Map<string, GroupedBackup>();
    for (const b of $backups) {
      let g = m.get(b.game_id);
      if (!g) {
        const det = gameById.get(b.game_id) ?? null;
        g = {
          game_id: b.game_id,
          name: det?.name ?? b.game_id,
          game: det,
          entries: [],
          activeCount: 0,
          restoredCount: 0,
          latestAt: b.created_at,
          oldestAt: b.created_at,
          sizeBytes: 0,
        };
        m.set(b.game_id, g);
      }
      g.entries.push(b);
      if (b.restored_at) g.restoredCount += 1;
      else g.activeCount += 1;
      if (b.created_at > g.latestAt) g.latestAt = b.created_at;
      if (b.created_at < g.oldestAt) g.oldestAt = b.created_at;
      g.sizeBytes += b.size_bytes ?? 0;
    }
    for (const g of m.values()) {
      g.entries.sort((a, b) => b.created_at.localeCompare(a.created_at));
    }
    return Array.from(m.values()).sort((a, b) => b.latestAt.localeCompare(a.latestAt));
  });

  let filtered = $derived.by<GroupedBackup[]>(() => {
    if (!query.trim()) return grouped;
    const q = query.trim().toLowerCase();
    return grouped
      .map((g) => ({
        ...g,
        entries: g.entries.filter(
          (e) =>
            g.name.toLowerCase().includes(q) ||
            e.dll_filename.toLowerCase().includes(q) ||
            (featureTitle(featureFromFamily(e.dll_family)).toLowerCase().includes(q)) ||
            (e.previous_version ?? "").toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.entries.length > 0);
  });

  let totalActive = $derived($backups.filter((b) => !b.restored_at).length);
  let totalRestored = $derived($backups.filter((b) => b.restored_at).length);
  let uniqueGames = $derived(new Set($backups.map((b) => b.game_id)).size);
  let totalBytes = $derived($backups.reduce((n, b) => n + (b.size_bytes ?? 0), 0));
  let oldestDate = $derived.by<string | null>(() => {
    if ($backups.length === 0) return null;
    return $backups.reduce((acc, b) => (b.created_at < acc ? b.created_at : acc), $backups[0].created_at);
  });
  let newestDate = $derived.by<string | null>(() => {
    if ($backups.length === 0) return null;
    return $backups.reduce((acc, b) => (b.created_at > acc ? b.created_at : acc), $backups[0].created_at);
  });

  async function doRestore(b: BackupEntry): Promise<void> {
    if (restoringId) return;
    restoringId = b.id;
    try {
      await restoreBackup(b.id);
      showToast("success", `Restored ${b.dll_filename}`);
      await loadBackups();
    } catch (err: unknown) {
      showToast("danger", `Restore failed: ${String(err)}`);
    } finally {
      restoringId = null;
    }
  }

  let selectedEntries = $derived.by<BackupEntry[]>(() => $backups.filter((b) => selectedIds.has(b.id)));
  let selectedActiveCount = $derived(selectedEntries.filter((e) => !e.restored_at).length);
  let selectedTotalBytes = $derived(selectedEntries.reduce((n, e) => n + (e.size_bytes ?? 0), 0));

  async function bulkRestore(): Promise<void> {
    if (bulkRunning) return;
    const targets = selectedEntries.filter((e) => !e.restored_at);
    if (targets.length === 0) {
      showToast("info", "Nothing to restore — selected entries are already restored");
      return;
    }
    bulkRunning = "restore";
    let ok = 0;
    let fail = 0;
    for (const e of targets) {
      try {
        await restoreBackup(e.id);
        ok += 1;
      } catch {
        fail += 1;
      }
    }
    bulkRunning = null;
    selectedIds = new Set();
    await loadBackups();
    if (fail === 0) showToast("success", `Restored ${ok} snapshot${ok === 1 ? "" : "s"}`);
    else if (ok === 0) showToast("danger", `Restore failed for all ${fail} snapshots`);
    else showToast("warning", `Restored ${ok}, ${fail} failed`);
  }

  async function bulkDelete(): Promise<void> {
    if (bulkRunning) return;
    if (selectedEntries.length === 0) return;
    const sizeLabel = selectedTotalBytes > 0 ? ` totalling ${fmtBytes(selectedTotalBytes)}` : "";
    const ok = await confirm(
      `Delete ${selectedEntries.length} backup snapshot${selectedEntries.length === 1 ? "" : "s"}${sizeLabel}?\n\nThis removes the snapshot files from disk. You will NOT be able to restore the originals afterwards. This action is irreversible.`,
      { title: "Delete backups", kind: "warning", okLabel: `Delete ${selectedEntries.length}`, cancelLabel: "Cancel" },
    );
    if (!ok) return;
    bulkRunning = "delete";
    let removed = 0;
    let fail = 0;
    for (const e of selectedEntries) {
      try {
        const outcome = await deleteBackup(e.id);
        if (outcome.file_error) fail += 1;
        else removed += 1;
      } catch {
        fail += 1;
      }
    }
    bulkRunning = null;
    selectedIds = new Set();
    await loadBackups();
    if (fail === 0) showToast("success", `Deleted ${removed} snapshot${removed === 1 ? "" : "s"}`);
    else if (removed === 0) showToast("danger", `Delete failed for all ${fail} snapshots`);
    else showToast("warning", `Deleted ${removed}, ${fail} failed`);
  }

  async function doDelete(b: BackupEntry): Promise<void> {
    if (deletingId) return;
    const sizeLabel = b.size_bytes ? ` (${fmtBytes(b.size_bytes)})` : "";
    const ok = await confirm(
      `Delete backup of ${b.dll_filename}${sizeLabel}?\n\nThis removes the snapshot file from disk. You will NOT be able to restore this DLL afterwards.`,
      { title: "Delete backup", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" },
    );
    if (!ok) return;
    deletingId = b.id;
    try {
      const outcome = await deleteBackup(b.id);
      if (outcome.file_error) {
        showToast("warning", `Row removed, file delete failed: ${outcome.file_error}`);
      } else {
        showToast("success", `Deleted ${b.dll_filename}`);
      }
      await loadBackups();
    } catch (err: unknown) {
      showToast("danger", `Delete failed: ${String(err)}`);
    } finally {
      deletingId = null;
    }
  }

  async function revealBackup(b: BackupEntry): Promise<void> {
    if (openingPath) return;
    openingPath = b.id;
    try {
      const { revealPath } = await import("../lib/api");
      await revealPath(b.backup_path);
    } catch (err: unknown) {
      showToast("danger", `Reveal failed: ${String(err)}`);
    } finally {
      openingPath = null;
    }
  }

  async function openGameFolder(g: GroupedBackup): Promise<void> {
    if (!g.game) {
      showToast("warning", "Game install path not available — game may have been uninstalled");
      return;
    }
    try {
      await openPath(g.game.install_dir);
    } catch (err: unknown) {
      showToast("danger", `Open folder: ${String(err)}`);
    }
  }

  function fmtDate(s: string): string {
    const d = new Date(s);
    if (isNaN(d.getTime())) return "—";
    return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  }
  function fmtDateShort(s: string): string {
    const d = new Date(s);
    if (isNaN(d.getTime())) return "—";
    return d.toISOString().slice(0, 10);
  }
  function shortSha(s: string | null): string {
    return s ? s.slice(0, 8) : "?";
  }
  function fmtBytes(n: number | null | undefined): string {
    if (n == null || n === 0) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    let v = n;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i += 1;
    }
    return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v).toString()} ${units[i]}`;
  }

  function expandAll(): void {
    const next: Record<string, boolean> = {};
    for (const g of filtered) next[g.game_id] = true;
    expanded = next;
  }
  function collapseAll(): void {
    expanded = {};
  }
</script>

<header class="view-header">
  <div>
    <h1 class="view-title">Backups</h1>
    <p class="view-subtitle">Every DLL replaced by DLSSync is snapshotted first. Restore any time, one click.</p>
  </div>
  <div class="header-actions">
    {#if filtered.length > 0}
      <button class="btn btn-ghost btn-sm" onclick={expandAll}>Expand all</button>
      <button class="btn btn-ghost btn-sm" onclick={collapseAll}>Collapse all</button>
    {/if}
  </div>
</header>

{#if $backups.length === 0}
  <div class="empty">
    <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
    <h3 class="empty-title">No backups yet</h3>
    <p class="section-sub">Backups are created automatically when you apply a DLL update. Open any game from the Library, pick DLLs, and click Apply selected.</p>
  </div>
{:else}
  <section class="backup-hero" in:fly={{ y: 6, duration: 220 }}>
    <div class="hero-stats">
      <div class="hero-stat is-primary">
        <span class="hero-num">{$backups.length.toLocaleString()}</span>
        <span class="hero-lbl">Total backups</span>
      </div>
      <div class="hero-stat">
        <span class="hero-num">{fmtBytes(totalBytes)}</span>
        <span class="hero-lbl">Disk used</span>
      </div>
      <div class="hero-stat">
        <span class="hero-num">{uniqueGames}</span>
        <span class="hero-lbl">Games covered</span>
      </div>
      <div class="hero-stat">
        <span class="hero-num is-update">{totalActive}</span>
        <span class="hero-lbl">Restorable</span>
      </div>
      <div class="hero-stat">
        <span class="hero-num is-success">{totalRestored}</span>
        <span class="hero-lbl">Already restored</span>
      </div>
      <div class="hero-stat is-meta">
        <span class="hero-num is-small mono">{newestDate ? fmtDateShort(newestDate) : "—"}</span>
        <span class="hero-lbl">Newest snapshot</span>
      </div>
      <div class="hero-stat is-meta">
        <span class="hero-num is-small mono">{oldestDate ? fmtDateShort(oldestDate) : "—"}</span>
        <span class="hero-lbl">Oldest snapshot</span>
      </div>
    </div>
  </section>

  {#if selectedIds.size > 0}
    <div class="bulk-bar" in:fly={{ y: -4, duration: 180 }}>
      <span class="bulk-count">{selectedIds.size} selected</span>
      {#if selectedTotalBytes > 0}
        <span class="bulk-meta">{fmtBytes(selectedTotalBytes)}</span>
      {/if}
      {#if selectedActiveCount > 0 && selectedActiveCount !== selectedIds.size}
        <span class="bulk-meta">{selectedActiveCount} restorable</span>
      {/if}
      <div class="bulk-spacer"></div>
      <button class="btn btn-sm btn-ghost" onclick={clearSelection} disabled={bulkRunning !== null}>Clear</button>
      <button class="btn btn-sm btn-accent" onclick={bulkRestore} disabled={selectedActiveCount === 0 || bulkRunning !== null}>
        {#if bulkRunning === "restore"}
          <span class="spin"></span>
          Restoring {selectedActiveCount}…
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
          Restore {selectedActiveCount}
        {/if}
      </button>
      <button class="btn btn-sm btn-danger-ghost" onclick={bulkDelete} disabled={bulkRunning !== null}>
        {#if bulkRunning === "delete"}
          <span class="spin"></span>
          Deleting {selectedIds.size}…
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>
          Delete {selectedIds.size}
        {/if}
      </button>
    </div>
  {/if}

  <div class="backup-toolbar">
    <div class="backup-search">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="search-icon"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
      <input
        type="search"
        placeholder="Search by game, DLL file, feature, or version…"
        bind:value={query}
      />
      {#if query}
        <button class="search-clear" onclick={() => (query = "")} aria-label="Clear search">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      {/if}
    </div>
    <span class="toolbar-summary">
      {filtered.reduce((a, g) => a + g.entries.length, 0)} of {$backups.length} backup{$backups.length === 1 ? "" : "s"}{filtered.length !== grouped.length ? ` · ${filtered.length} game${filtered.length === 1 ? "" : "s"}` : ""}
    </span>
  </div>

  {#if filtered.length === 0}
    <div class="empty small">
      <p class="section-sub">No backups match your search.</p>
      <button class="btn btn-accent" onclick={() => (query = "")}>Clear search</button>
    </div>
  {:else}
    <div class="groups">
      {#each filtered as g, i (g.game_id)}
        {@const accent = g.game ? launcherAccent(g.game.launcher) : DEFAULT_VENDOR_ACCENT}
        {@const groupSel = groupSelectionState(g)}
        <section class="group" in:fly={{ y: 6, duration: 260, delay: 40 + i * 30 }}>
          <div class="group-row">
            <label class="group-check" title={groupSel === "all" ? "Deselect all in this game" : "Select all in this game"}>
              <input
                type="checkbox"
                checked={groupSel === "all"}
                indeterminate={groupSel === "some"}
                onchange={(e) => toggleGroupSelection(g, (e.target as HTMLInputElement).checked)}
              />
              <span class="check-box"></span>
            </label>
          <button
            class="group-head"
            onclick={() => (expanded = { ...expanded, [g.game_id]: !expanded[g.game_id] })}
            aria-expanded={!!expanded[g.game_id]}
          >
            <div class="group-thumb" style:--launcher-accent={accent}>
              {#if g.game?.image_url}
                <img src={g.game.image_url} alt={g.name} loading="lazy" />
              {:else}
                <span class="thumb-fallback">{g.name.slice(0, 1).toUpperCase()}</span>
              {/if}
            </div>
            <div class="group-meta">
              <div class="group-name-row">
                <span class="group-name">{g.name}</span>
                {#if g.game}
                  <span class="chip chip-neutral group-launcher">{launcherLabel(g.game.launcher)}</span>
                {/if}
              </div>
              <div class="group-stats">
                <span class="stat-line"><strong>{g.entries.length}</strong> snapshot{g.entries.length === 1 ? "" : "s"}</span>
                {#if g.activeCount > 0}<span class="dot"></span><span class="stat-line is-update">{g.activeCount} restorable</span>{/if}
                {#if g.restoredCount > 0}<span class="dot"></span><span class="stat-line is-success">{g.restoredCount} restored</span>{/if}
                {#if g.sizeBytes > 0}<span class="dot"></span><span class="stat-line">{fmtBytes(g.sizeBytes)}</span>{/if}
                <span class="dot"></span><span class="stat-line">latest {fmtDateShort(g.latestAt)}</span>
              </div>
            </div>
            <svg
              class="chevron"
              class:open={expanded[g.game_id]}
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            ><polyline points="9 18 15 12 9 6"/></svg>
          </button>
          </div>
          {#if expanded[g.game_id]}
            <div class="group-actions">
              {#if g.game}
                <button class="btn btn-sm btn-ghost" onclick={() => openGameFolder(g)} title="Open install folder">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                  Open install folder
                </button>
              {/if}
            </div>
            <ul class="entries">
              {#each g.entries as b (b.id)}
                {@const fSlot = featureFromFamily(b.dll_family)}
                <li class="entry" class:restored={b.restored_at} class:is-selected={selectedIds.has(b.id)}>
                  <label class="entry-check" title="Select for bulk restore/delete">
                    <input
                      type="checkbox"
                      checked={selectedIds.has(b.id)}
                      onchange={() => toggleEntry(b.id)}
                    />
                    <span class="check-box"></span>
                  </label>
                  <div class="entry-glyph" aria-hidden="true">
                    <FeatureIcon id={featureIconId(fSlot)} size={16} />
                  </div>
                  <div class="entry-main">
                    <div class="entry-head">
                      <span class="entry-title">{featureTitle(fSlot)}</span>
                      {#if b.restored_at}
                        <span class="chip chip-success small-chip" title={`Restored ${fmtDate(b.restored_at)}`}>Restored</span>
                      {:else}
                        <span class="chip chip-update small-chip">Active backup</span>
                      {/if}
                    </div>
                    <div class="entry-meta mono">
                      <span class="file">{b.dll_filename}</span>
                      <span class="sep">·</span>
                      <span>v{b.previous_version ?? "?"}</span>
                      <span class="sep">·</span>
                      <span title={b.previous_sha256 ?? ""}>sha {shortSha(b.previous_sha256)}</span>
                      <span class="sep">·</span>
                      <span>{fmtBytes(b.size_bytes)}</span>
                      <span class="sep">·</span>
                      <span title={b.created_at}>{fmtDate(b.created_at)}</span>
                    </div>
                    <div class="entry-path mono truncate" title={b.original_path}>↳ {b.original_path}</div>
                  </div>
                  <div class="entry-actions">
                    <button
                      class="btn btn-sm btn-ghost"
                      onclick={() => revealBackup(b)}
                      title="Reveal snapshot file"
                      disabled={openingPath === b.id}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                    </button>
                    <button
                      class="btn btn-sm btn-ghost btn-danger-ghost"
                      onclick={() => doDelete(b)}
                      title="Delete backup snapshot from disk"
                      disabled={deletingId === b.id}
                    >
                      {#if deletingId === b.id}
                        <span class="spin"></span>
                      {:else}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>
                      {/if}
                    </button>
                    <button
                      class="btn btn-sm btn-accent"
                      disabled={!!b.restored_at || restoringId === b.id}
                      onclick={() => doRestore(b)}
                    >
                      {#if restoringId === b.id}
                        <span class="spin"></span>
                        Restoring
                      {:else if b.restored_at}
                        Restored
                      {:else}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
                        Restore
                      {/if}
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .view-header {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
    flex-wrap: wrap;
  }
  .view-header > div:first-child { flex: 1 1 240px; min-width: 0; }
  .header-actions { display: flex; gap: 6px; flex-wrap: wrap; flex-shrink: 0; }
  .empty {
    padding: 80px 0;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }
  .empty.small { padding: 40px 0; }
  .empty :global(svg) { opacity: 0.4; margin-bottom: 8px; }
  .empty-title { font-size: var(--fs-lg); font-weight: 600; color: var(--text-primary); }
  .empty .section-sub { max-width: 460px; }

  .backup-hero {
    margin-bottom: 16px;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }
  .hero-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
  }
  .hero-stat {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition: border-color var(--dur-fast) var(--ease);
  }
  .hero-stat:hover { border-color: var(--border-hover); }
  .hero-stat.is-primary { border-color: var(--accent-ring); background: var(--accent-soft); }
  .hero-stat.is-meta { background: transparent; border-color: var(--border); }
  .hero-num {
    font-size: var(--fs-2xl);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tighter);
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }
  .hero-num.is-update { color: var(--update); }
  .hero-num.is-success { color: var(--success); }
  .hero-num.is-small { font-size: var(--fs-md); font-weight: 600; letter-spacing: 0; }
  .hero-lbl {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }

  .backup-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .backup-search { position: relative; flex: 1; max-width: 520px; display: flex; align-items: center; }
  .backup-search input {
    width: 100%;
    padding: 9px 34px 9px 34px;
    border-radius: var(--radius-full);
    font-size: var(--fs-sm);
    background: var(--bg-input);
    border: 1px solid var(--border);
  }
  .backup-search input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-dim); }
  .backup-search .search-icon { position: absolute; left: 12px; color: var(--text-muted); pointer-events: none; }
  .search-clear {
    position: absolute;
    right: 8px;
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    border-radius: var(--radius-full);
  }
  .search-clear:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .toolbar-summary { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .groups { display: flex; flex-direction: column; gap: 10px; }
  .group {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: border-color 0.15s var(--ease);
  }
  .group:hover { border-color: var(--border-hover); }
  .group-head {
    display: grid;
    grid-template-columns: 56px 1fr auto;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 12px 16px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s var(--ease);
  }
  .group-head:hover { background: var(--bg-card-hover); }
  .group-thumb {
    width: 56px;
    height: 28px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-art-fallback);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .group-thumb img { width: 100%; height: 100%; object-fit: cover; }
  .thumb-fallback {
    font-size: var(--fs-md);
    font-weight: 700;
    color: var(--launcher-accent, var(--accent));
    opacity: 0.7;
  }
  .group-meta { min-width: 0; }
  .group-name-row { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .group-name { font-size: var(--fs-md); font-weight: 600; letter-spacing: var(--letter-tight); }
  .group-launcher { font-size: var(--fs-2xs); padding: 1px 7px; }
  .group-stats { display: flex; align-items: center; gap: 6px; font-size: var(--fs-xs); color: var(--text-muted); flex-wrap: wrap; }
  .group-stats strong { color: var(--text-secondary); font-weight: 600; }
  .stat-line.is-update { color: var(--update); }
  .stat-line.is-success { color: var(--success); }
  .group-stats .dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.4;
  }
  .chevron { transition: transform 0.2s var(--ease); color: var(--text-muted); }
  .chevron.open { transform: rotate(90deg); color: var(--accent); }

  .group-actions {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-input);
    display: flex;
    gap: 8px;
  }
  .entries { list-style: none; padding: 0; margin: 0; }
  .entry {
    display: grid;
    grid-template-columns: 28px 32px 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    transition: background 0.12s var(--ease);
  }
  .entry.is-selected { background: var(--accent-soft); }
  .entry-check, .group-check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    user-select: none;
  }
  .entry-check input, .group-check input { position: absolute; opacity: 0; pointer-events: none; }
  .entry-check .check-box, .group-check .check-box {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1.5px solid var(--border-strong);
    background: var(--bg-input);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s var(--ease), border-color 0.12s var(--ease);
  }
  .entry-check input:checked + .check-box, .group-check input:checked + .check-box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .entry-check input:checked + .check-box::after, .group-check input:checked + .check-box::after {
    content: "";
    width: 8px;
    height: 4px;
    border-left: 2px solid var(--accent-fg);
    border-bottom: 2px solid var(--accent-fg);
    transform: translate(0, -1px) rotate(-45deg);
  }
  .entry-check input:indeterminate + .check-box, .group-check input:indeterminate + .check-box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .entry-check input:indeterminate + .check-box::after, .group-check input:indeterminate + .check-box::after {
    content: "";
    width: 8px;
    height: 2px;
    background: var(--accent-fg);
    border-radius: 1px;
  }

  .group-row {
    display: grid;
    grid-template-columns: 36px 1fr;
    align-items: stretch;
  }
  .group-check { padding-left: 14px; padding-right: 0; }

  .bulk-bar {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    margin-bottom: 12px;
    background: var(--accent-soft);
    border: 1px solid var(--accent);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 14px rgba(0,0,0,0.25);
    backdrop-filter: blur(8px);
  }
  .bulk-count {
    font-size: var(--fs-sm);
    font-weight: 700;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .bulk-meta { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .bulk-spacer { flex: 1; }
  .entry:hover { background: var(--bg-card-hover); }
  .entry-glyph {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .entry.restored .entry-glyph { background: var(--success-dim); color: var(--success); }
  .entry-main { min-width: 0; }
  .entry-head { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .entry-title { font-size: var(--fs-sm); font-weight: 600; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .small-chip { padding: 1px 7px; font-size: var(--fs-2xs); letter-spacing: 0.04em; }
  .entry-meta { font-size: var(--fs-xs); color: var(--text-muted); display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .entry-meta .file { color: var(--text-secondary); }
  .entry-meta .sep { opacity: 0.4; }
  .entry-path { font-size: var(--fs-2xs); color: var(--text-muted); opacity: 0.7; margin-top: 3px; }
  .entry.restored .entry-main { opacity: 0.7; }
  .entry-actions { display: inline-flex; gap: 6px; flex-shrink: 0; }

  .spin { width: 11px; height: 11px; border: 2px solid currentColor; border-top-color: transparent; border-radius: 50%; animation: spin 0.7s linear infinite; display: inline-block; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
