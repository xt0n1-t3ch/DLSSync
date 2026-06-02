<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { backups, loadBackups, games, showToast, settings, persistSettings } from "../lib/stores";
  import { pushNotification, makeNotificationEntry } from "../lib/notifications";
  import { restoreBackup, restoreSystemDriver, deleteBackup, openPath, type BackupEntry, type DetectedGame, type BackupsGroupBy } from "../lib/api";
  import { BACKUPS_GROUP_BYS, BACKUPS_GROUP_BY_DEFAULT } from "../lib/ux";
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
  import { get } from "svelte/store";
  import { t, locale, translate } from "../lib/i18n/index";

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
    missingCount: number;
    latestAt: string;
    oldestAt: string;
    sizeBytes: number;
  };

  function isMissing(b: BackupEntry): boolean {
    return b.size_bytes == null;
  }

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

  let groupBy: BackupsGroupBy = $derived(
    ($settings?.ui_prefs.backups_group_by ?? BACKUPS_GROUP_BY_DEFAULT) as BackupsGroupBy,
  );

  async function setGroupBy(mode: BackupsGroupBy): Promise<void> {
    if (!$settings || !BACKUPS_GROUP_BYS.includes(mode)) return;
    await persistSettings({ ...$settings, ui_prefs: { ...$settings.ui_prefs, backups_group_by: mode } });
  }

  function dateLabel(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, { weekday: "short", year: "numeric", month: "short", day: "numeric" });
  }

  let dllBackups = $derived($backups.filter((b) => b.backup_type !== "driver_package"));
  let driverBackups = $derived($backups.filter((b) => b.backup_type === "driver_package"));

  let driverQuery = $state("");
  let driverClassFilter = $state<string>("all");
  let restoringDriverId = $state<string | null>(null);

  type DriverClassGroup = { deviceClass: string; entries: BackupEntry[] };

  let driverClasses = $derived.by<string[]>(() => {
    const set = new Set<string>();
    for (const b of driverBackups) set.add(b.device_class ?? "Driver");
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  });

  let driverGroups = $derived.by<DriverClassGroup[]>(() => {
    const q = driverQuery.trim().toLowerCase();
    const m = new Map<string, BackupEntry[]>();
    for (const b of driverBackups) {
      const cls = b.device_class ?? "Driver";
      if (driverClassFilter !== "all" && cls !== driverClassFilter) continue;
      if (
        q &&
        !(
          cls.toLowerCase().includes(q) ||
          (b.driver_provider ?? "").toLowerCase().includes(q) ||
          b.dll_filename.toLowerCase().includes(q) ||
          (b.hardware_id ?? "").toLowerCase().includes(q) ||
          (b.previous_version ?? "").toLowerCase().includes(q)
        )
      )
        continue;
      (m.get(cls) ?? m.set(cls, []).get(cls)!).push(b);
    }
    return Array.from(m.entries())
      .map(([deviceClass, entries]) => ({
        deviceClass,
        entries: entries.sort((a, b) => b.created_at.localeCompare(a.created_at)),
      }))
      .sort((a, b) => a.deviceClass.localeCompare(b.deviceClass));
  });

  async function doDriverRestore(b: BackupEntry): Promise<void> {
    if (restoringDriverId) return;
    const loc = get(locale);
    const ok = await confirm(
      translate(loc, "view.backups.driver.confirmRollback.body", {
        provider: b.driver_provider ?? translate(loc, "view.backups.driver.thisDriver"),
        deviceClass: (b.device_class ?? translate(loc, "view.backups.driver.driverWord")).toLowerCase(),
        version: b.previous_version ?? "?",
      }),
      {
        title: translate(loc, "view.backups.driver.confirmRollback.title"),
        kind: "warning",
        okLabel: translate(loc, "view.backups.driver.rollBack"),
        cancelLabel: translate(loc, "view.backups.cancel"),
      },
    );
    if (!ok) return;
    restoringDriverId = b.id;
    try {
      const outcome = await restoreSystemDriver(b.id);
      if (outcome.success) {
        showToast(
          "success",
          translate(loc, "view.backups.driver.toastRolledBack", { file: b.dll_filename }) +
            (outcome.reboot_required ? translate(loc, "view.backups.driver.rebootSuffix") : ""),
        );
        void pushNotification(
          makeNotificationEntry(
            "backup_restored",
            translate(loc, "view.backups.driver.notifTitle", {
              deviceClass: b.device_class ?? translate(loc, "view.backups.driver.driverWord"),
            }),
            `${b.driver_provider ?? ""} ${b.dll_filename}`.trim(),
          ),
        ).catch((err) => console.warn("[dlssync] push driver-rollback notification failed:", err));
        await loadBackups();
      } else {
        showToast("danger", outcome.message);
      }
    } catch (err: unknown) {
      showToast("danger", translate(loc, "view.backups.driver.toastRollbackFailed", { error: String(err) }));
    } finally {
      restoringDriverId = null;
    }
  }

  let grouped = $derived.by<GroupedBackup[]>(() => {
    const m = new Map<string, GroupedBackup>();
    for (const b of dllBackups) {
      const key = groupBy === "date" ? b.created_at.slice(0, 10) : b.game_id;
      let g = m.get(key);
      if (!g) {
        const det = groupBy === "game" ? gameById.get(b.game_id) ?? null : null;
        const name = groupBy === "date" ? dateLabel(b.created_at) : det?.name ?? b.game_id;
        g = {
          game_id: key,
          name,
          game: det,
          entries: [],
          activeCount: 0,
          restoredCount: 0,
          missingCount: 0,
          latestAt: b.created_at,
          oldestAt: b.created_at,
          sizeBytes: 0,
        };
        m.set(key, g);
      }
      g.entries.push(b);
      if (isMissing(b)) g.missingCount += 1;
      if (b.restored_at) g.restoredCount += 1;
      else if (!isMissing(b)) g.activeCount += 1;
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

  let totalRestorable = $derived(dllBackups.filter((b) => !b.restored_at && !isMissing(b)).length);
  let totalRestored = $derived(dllBackups.filter((b) => b.restored_at).length);
  let totalMissing = $derived(dllBackups.filter((b) => isMissing(b)).length);
  let uniqueGames = $derived(new Set(dllBackups.map((b) => b.game_id)).size);
  let totalBytes = $derived(dllBackups.reduce((n, b) => n + (b.size_bytes ?? 0), 0));
  let oldestDate = $derived.by<string | null>(() => {
    if (dllBackups.length === 0) return null;
    return dllBackups.reduce((acc, b) => (b.created_at < acc ? b.created_at : acc), dllBackups[0].created_at);
  });
  let newestDate = $derived.by<string | null>(() => {
    if (dllBackups.length === 0) return null;
    return dllBackups.reduce((acc, b) => (b.created_at > acc ? b.created_at : acc), dllBackups[0].created_at);
  });

  async function doRestore(b: BackupEntry): Promise<void> {
    if (restoringId) return;
    const loc = get(locale);
    restoringId = b.id;
    try {
      await restoreBackup(b.id);
      showToast("success", translate(loc, "view.backups.toastRestoredFile", { file: b.dll_filename }));
      void pushNotification(
        makeNotificationEntry(
          "backup_restored",
          translate(loc, "view.backups.toastRestoredFile", { file: b.dll_filename }),
          gameById.get(b.game_id)?.name ?? b.game_id,
          { game_id: b.game_id },
        ),
      ).catch((err) => console.warn("[dlssync] push backup-restored notification failed:", err));
      await loadBackups();
    } catch (err: unknown) {
      showToast("danger", translate(loc, "view.backups.toastRestoreFailed", { error: String(err) }));
    } finally {
      restoringId = null;
    }
  }

  let selectedEntries = $derived.by<BackupEntry[]>(() => dllBackups.filter((b) => selectedIds.has(b.id)));
  let selectedActiveCount = $derived(selectedEntries.filter((e) => !e.restored_at && !isMissing(e)).length);
  let selectedTotalBytes = $derived(selectedEntries.reduce((n, e) => n + (e.size_bytes ?? 0), 0));

  async function bulkRestore(): Promise<void> {
    if (bulkRunning) return;
    const loc = get(locale);
    const targets = selectedEntries.filter((e) => !e.restored_at && !isMissing(e));
    if (targets.length === 0) {
      showToast("info", translate(loc, "view.backups.toastNothingToRestore"));
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
    if (ok > 0) {
      void pushNotification(
        makeNotificationEntry(
          "backup_restored",
          translate(loc, "view.backups.toastRestoredCount", { count: ok }),
          fail > 0
            ? translate(loc, "view.backups.toastFailedToRestore", { count: fail })
            : translate(loc, "view.backups.toastAllRestored"),
        ),
      ).catch((err) => console.warn("[dlssync] push backup-restored notification failed:", err));
    }
    if (fail === 0) showToast("success", translate(loc, "view.backups.toastRestoredCount", { count: ok }));
    else if (ok === 0) showToast("danger", translate(loc, "view.backups.toastRestoreFailedAll", { count: fail }));
    else showToast("warning", translate(loc, "view.backups.toastRestorePartial", { ok, fail }));
  }

  async function bulkDelete(): Promise<void> {
    if (bulkRunning) return;
    if (selectedEntries.length === 0) return;
    const loc = get(locale);
    const sizeLabel =
      selectedTotalBytes > 0
        ? translate(loc, "view.backups.confirmDeleteMany.sizeSuffix", { size: fmtBytes(selectedTotalBytes) })
        : "";
    const ok = await confirm(
      translate(loc, "view.backups.confirmDeleteMany.body", {
        count: selectedEntries.length,
        sizeSuffix: sizeLabel,
      }),
      {
        title: translate(loc, "view.backups.confirmDeleteMany.title"),
        kind: "warning",
        okLabel: translate(loc, "view.backups.deleteCount", { count: selectedEntries.length }),
        cancelLabel: translate(loc, "view.backups.cancel"),
      },
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
    if (fail === 0) showToast("success", translate(loc, "view.backups.toastDeletedCount", { count: removed }));
    else if (removed === 0) showToast("danger", translate(loc, "view.backups.toastDeleteFailedAll", { count: fail }));
    else showToast("warning", translate(loc, "view.backups.toastDeletePartial", { removed, fail }));
  }

  async function doDelete(b: BackupEntry): Promise<void> {
    if (deletingId) return;
    const loc = get(locale);
    const sizeLabel = b.size_bytes
      ? translate(loc, "view.backups.confirmDeleteOne.sizeSuffix", { size: fmtBytes(b.size_bytes) })
      : "";
    const ok = await confirm(
      translate(loc, "view.backups.confirmDeleteOne.body", {
        file: b.dll_filename,
        sizeSuffix: sizeLabel,
      }),
      {
        title: translate(loc, "view.backups.confirmDeleteOne.title"),
        kind: "warning",
        okLabel: translate(loc, "view.backups.delete"),
        cancelLabel: translate(loc, "view.backups.cancel"),
      },
    );
    if (!ok) return;
    deletingId = b.id;
    try {
      const outcome = await deleteBackup(b.id);
      if (outcome.file_error) {
        showToast("warning", translate(loc, "view.backups.toastRowRemovedFileFailed", { error: outcome.file_error }));
      } else {
        showToast("success", translate(loc, "view.backups.toastDeletedFile", { file: b.dll_filename }));
      }
      await loadBackups();
    } catch (err: unknown) {
      showToast("danger", translate(loc, "view.backups.toastDeleteFailed", { error: String(err) }));
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
      showToast("danger", translate(get(locale), "view.backups.toastRevealFailed", { error: String(err) }));
    } finally {
      openingPath = null;
    }
  }

  async function openGameFolder(g: GroupedBackup): Promise<void> {
    const loc = get(locale);
    if (!g.game) {
      showToast("warning", translate(loc, "view.backups.toastInstallPathUnavailable"));
      return;
    }
    try {
      await openPath(g.game.install_dir);
    } catch (err: unknown) {
      showToast("danger", translate(loc, "view.backups.toastOpenFolderFailed", { error: String(err) }));
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
    <h1 class="view-title">{$t("view.backups.title")}</h1>
    <p class="view-subtitle">{$t("view.backups.subtitle")}</p>
  </div>
  <div class="header-actions">
    {#if filtered.length > 0}
      <button class="btn btn-ghost btn-sm" onclick={expandAll}>{$t("view.backups.expandAll")}</button>
      <button class="btn btn-ghost btn-sm" onclick={collapseAll}>{$t("view.backups.collapseAll")}</button>
    {/if}
  </div>
</header>

{#if $backups.length === 0}
  <div class="empty">
    <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
    <h3 class="empty-title">{$t("view.backups.emptyTitle")}</h3>
    <p class="section-sub">{$t("view.backups.emptyBody")}</p>
  </div>
{:else}
  {#if dllBackups.length > 0}
  <section class="backup-hero aura-card edge-accent" in:fly={{ y: 6, duration: 220 }}>
    <div class="hero-stats">
      <div class="bk-stat">
        <span class="bk-stat-badge aura-badge" data-tint="blue" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
        </span>
        <div class="bk-stat-text">
          <span class="bk-stat-num">{dllBackups.length.toLocaleString()}</span>
          <span class="bk-stat-lbl">{$t("view.backups.kpi.total")}</span>
        </div>
      </div>
      <div class="bk-stat">
        <span class="bk-stat-badge aura-badge" data-tint="purple" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" y1="12" x2="2" y2="12"/><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" y1="16" x2="6.01" y2="16"/><line x1="10" y1="16" x2="10.01" y2="16"/></svg>
        </span>
        <div class="bk-stat-text">
          <span class="bk-stat-num">{fmtBytes(totalBytes)}</span>
          <span class="bk-stat-lbl">{$t("view.backups.kpi.diskUsed")}</span>
        </div>
      </div>
      <div class="bk-stat">
        <span class="bk-stat-badge aura-badge" data-tint="orange" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
        </span>
        <div class="bk-stat-text">
          <span class="bk-stat-num is-update">{totalRestorable}</span>
          <span class="bk-stat-lbl">{$t("view.backups.kpi.restorable")}</span>
        </div>
      </div>
      <div class="bk-stat">
        <span class="bk-stat-badge aura-badge" data-tint="green" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
        </span>
        <div class="bk-stat-text">
          <span class="bk-stat-num is-success">{totalRestored}</span>
          <span class="bk-stat-lbl">{$t("view.backups.kpi.alreadyRestored")}</span>
        </div>
      </div>
    </div>
    <div class="hero-meta-strip">
      <span class="hero-meta-item">
        <span class="hero-meta-label">{$t("view.backups.meta.gamesCovered")}</span>
        <span class="hero-meta-value">{uniqueGames}</span>
      </span>
      <span class="hero-meta-sep"></span>
      <span class="hero-meta-item">
        <span class="hero-meta-label">{$t("view.backups.meta.newest")}</span>
        <span class="hero-meta-value mono">{newestDate ? fmtDateShort(newestDate) : "—"}</span>
      </span>
      <span class="hero-meta-sep"></span>
      <span class="hero-meta-item">
        <span class="hero-meta-label">{$t("view.backups.meta.oldest")}</span>
        <span class="hero-meta-value mono">{oldestDate ? fmtDateShort(oldestDate) : "—"}</span>
      </span>
      {#if totalMissing > 0}
        <span class="hero-meta-sep"></span>
        <span class="hero-meta-item hero-meta-warn" title={$t("view.backups.meta.missingHint")}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          <span class="hero-meta-value">{$t("view.backups.meta.missingCount", { count: totalMissing })}</span>
        </span>
      {/if}
    </div>
  </section>

  {#if selectedIds.size > 0}
    <div class="bulk-bar" in:fly={{ y: -4, duration: 180 }}>
      <span class="bulk-count">{$t("view.backups.bulk.selected", { count: selectedIds.size })}</span>
      {#if selectedTotalBytes > 0}
        <span class="bulk-meta">{fmtBytes(selectedTotalBytes)}</span>
      {/if}
      {#if selectedActiveCount > 0 && selectedActiveCount !== selectedIds.size}
        <span class="bulk-meta">{$t("view.backups.bulk.restorable", { count: selectedActiveCount })}</span>
      {/if}
      <div class="bulk-spacer"></div>
      <button class="btn btn-sm btn-ghost" onclick={clearSelection} disabled={bulkRunning !== null}>{$t("view.backups.bulk.clear")}</button>
      <button class="btn btn-sm btn-accent" onclick={bulkRestore} disabled={selectedActiveCount === 0 || bulkRunning !== null}>
        {#if bulkRunning === "restore"}
          <span class="spin"></span>
          {$t("view.backups.bulk.restoring", { count: selectedActiveCount })}
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
          {$t("view.backups.bulk.restore", { count: selectedActiveCount })}
        {/if}
      </button>
      <button class="btn btn-sm btn-danger-ghost" onclick={bulkDelete} disabled={bulkRunning !== null}>
        {#if bulkRunning === "delete"}
          <span class="spin"></span>
          {$t("view.backups.bulk.deleting", { count: selectedIds.size })}
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>
          {$t("view.backups.bulk.delete", { count: selectedIds.size })}
        {/if}
      </button>
    </div>
  {/if}

  <div class="backup-toolbar">
    <div class="backup-search">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="search-icon"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
      <input
        type="search"
        placeholder={$t("view.backups.searchPlaceholder")}
        bind:value={query}
      />
      {#if query}
        <button class="search-clear" onclick={() => (query = "")} aria-label={$t("view.backups.clearSearchAria")}>
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      {/if}
    </div>
    <div class="group-by-toggle" role="group" aria-label={$t("view.backups.groupByAria")}>
      <button class="seg-btn" class:active={groupBy === "game"} onclick={() => void setGroupBy("game")} aria-pressed={groupBy === "game"}>{$t("view.backups.groupBy.game")}</button>
      <button class="seg-btn" class:active={groupBy === "date"} onclick={() => void setGroupBy("date")} aria-pressed={groupBy === "date"}>{$t("view.backups.groupBy.date")}</button>
    </div>
    <span class="toolbar-summary">
      {$t("view.backups.summaryCount", { shown: filtered.reduce((a, g) => a + g.entries.length, 0), count: dllBackups.length })}{filtered.length !== grouped.length ? ` · ${groupBy === "date" ? $t("view.backups.summaryDays", { count: filtered.length }) : $t("view.backups.summaryGames", { count: filtered.length })}` : ""}
    </span>
  </div>

  {#if filtered.length === 0}
    <div class="empty small">
      <p class="section-sub">{$t("view.backups.noMatch")}</p>
      <button class="btn btn-accent" onclick={() => (query = "")}>{$t("view.backups.clearSearch")}</button>
    </div>
  {:else}
    <div class="groups">
      {#each filtered as g, i (g.game_id)}
        {@const accent = g.game ? launcherAccent(g.game.launcher) : DEFAULT_VENDOR_ACCENT}
        {@const groupSel = groupSelectionState(g)}
        <section class="group" in:fly={{ y: 6, duration: 260, delay: 40 + i * 30 }}>
          <div class="group-row">
            <label class="group-check" title={groupSel === "all" ? $t("view.backups.group.deselectAll") : $t("view.backups.group.selectAll")}>
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
                <span class="stat-line">{@html $t("view.backups.group.snapshots", { count: g.entries.length })}</span>
                {#if g.activeCount > 0}<span class="dot"></span><span class="stat-line is-update">{$t("view.backups.group.restorable", { count: g.activeCount })}</span>{/if}
                {#if g.restoredCount > 0}<span class="dot"></span><span class="stat-line is-success">{$t("view.backups.group.restored", { count: g.restoredCount })}</span>{/if}
                {#if g.missingCount > 0}<span class="dot"></span><span class="stat-line is-missing">{$t("view.backups.group.missing", { count: g.missingCount })}</span>{/if}
                {#if g.sizeBytes > 0}<span class="dot"></span><span class="stat-line">{fmtBytes(g.sizeBytes)}</span>{/if}
                <span class="dot"></span><span class="stat-line">{$t("view.backups.group.latest", { date: fmtDateShort(g.latestAt) })}</span>
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
                <button class="btn btn-sm btn-ghost" onclick={() => openGameFolder(g)} title={$t("view.backups.openInstallFolder")}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                  {$t("view.backups.openInstallFolder")}
                </button>
              {/if}
            </div>
            <ul class="entries">
              {#each g.entries as b (b.id)}
                {@const fSlot = featureFromFamily(b.dll_family)}
                {@const missing = isMissing(b)}
                <li class="entry" class:restored={b.restored_at} class:missing class:is-selected={selectedIds.has(b.id)}>
                  <label class="entry-check" title={$t("view.backups.entry.selectHint")}>
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
                      {#if missing}
                        <span class="chip chip-danger small-chip" title={$t("view.backups.entry.missingHint")}>{$t("view.backups.entry.snapshotMissing")}</span>
                      {:else if b.restored_at}
                        <span class="chip chip-success small-chip" title={$t("view.backups.entry.restoredHint", { date: fmtDate(b.restored_at) })}>{$t("view.backups.entry.restored")}</span>
                      {:else}
                        <span class="chip chip-update small-chip" title={$t("view.backups.entry.activeHint")}>{$t("view.backups.entry.activeBackup")}</span>
                      {/if}
                    </div>
                    <div class="entry-meta mono">
                      <span class="file">{b.dll_filename}</span>
                      <span class="sep">·</span>
                      <span>v{b.previous_version ?? "?"}</span>
                      <span class="sep">·</span>
                      <span title={b.previous_sha256 ?? ""}>sha {shortSha(b.previous_sha256)}</span>
                      <span class="sep">·</span>
                      <span class:is-missing={missing}>{missing ? $t("view.backups.entry.gone") : fmtBytes(b.size_bytes)}</span>
                      <span class="sep">·</span>
                      <span title={b.created_at}>{fmtDate(b.created_at)}</span>
                    </div>
                    <div class="entry-path mono truncate" title={b.original_path}>↳ {b.original_path}</div>
                  </div>
                  <div class="entry-actions">
                    <button
                      class="btn btn-sm btn-ghost"
                      onclick={() => revealBackup(b)}
                      title={missing ? $t("view.backups.entry.revealMissing") : $t("view.backups.entry.reveal")}
                      disabled={openingPath === b.id}
                    >
                      {#if openingPath === b.id}
                        <span class="spin"></span>
                      {:else}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                      {/if}
                    </button>
                    <button
                      class="btn btn-sm btn-ghost btn-danger-ghost"
                      onclick={() => doDelete(b)}
                      title={$t("view.backups.entry.deleteHint")}
                      disabled={deletingId === b.id}
                    >
                      {#if deletingId === b.id}
                        <span class="spin"></span>
                      {:else}
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>
                      {/if}
                    </button>
                    {#if missing}
                      <button class="btn btn-sm btn-ghost" disabled title={$t("view.backups.entry.unavailableHint")}>
                        {$t("view.backups.entry.unavailable")}
                      </button>
                    {:else if b.restored_at}
                      <button
                        class="btn btn-sm btn-ghost"
                        disabled={restoringId === b.id}
                        onclick={() => doRestore(b)}
                        title={$t("view.backups.entry.restoreAgainHint")}
                      >
                        {#if restoringId === b.id}
                          <span class="spin"></span>
                          {$t("view.backups.entry.restoring")}
                        {:else}
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
                          {$t("view.backups.entry.restoreAgain")}
                        {/if}
                      </button>
                    {:else}
                      <button
                        class="btn btn-sm btn-accent"
                        disabled={restoringId === b.id}
                        onclick={() => doRestore(b)}
                        title={$t("view.backups.entry.restoreHint")}
                      >
                        {#if restoringId === b.id}
                          <span class="spin"></span>
                          {$t("view.backups.entry.restoring")}
                        {:else}
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
                          {$t("view.backups.entry.restore")}
                        {/if}
                      </button>
                    {/if}
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

  {#if driverBackups.length > 0}
    <section class="driver-section" in:fly={{ y: 6, duration: 220 }}>
      <div class="section-head driver-head">
        <h2 class="section-title">{$t("view.backups.driver.sectionTitle")}</h2>
        <span class="section-count">{driverBackups.length}</span>
      </div>
      <p class="section-sub driver-sub">
        {$t("view.backups.driver.sectionSub")}
      </p>

      <div class="driver-toolbar">
        <div class="backup-search">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="search-icon"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <input type="search" placeholder={$t("view.backups.driver.searchPlaceholder")} bind:value={driverQuery} />
          {#if driverQuery}
            <button class="search-clear" onclick={() => (driverQuery = "")} aria-label={$t("view.backups.clearSearchAria")}>
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          {/if}
        </div>
        {#if driverClasses.length > 1}
          <div class="pills" role="group" aria-label={$t("view.backups.driver.filterAria")}>
            <button class="pill" class:active={driverClassFilter === "all"} onclick={() => (driverClassFilter = "all")}>{$t("view.backups.driver.filterAll")}</button>
            {#each driverClasses as c (c)}
              <button class="pill" class:active={driverClassFilter === c} onclick={() => (driverClassFilter = c)}>{c}</button>
            {/each}
          </div>
        {/if}
      </div>

      {#if driverGroups.length === 0}
        <div class="empty small">
          <p class="section-sub">{$t("view.backups.driver.noMatch")}</p>
        </div>
      {:else}
        <div class="groups">
          {#each driverGroups as dg (dg.deviceClass)}
            <section class="group">
              <div class="driver-group-head">
                <span class="driver-class">{dg.deviceClass}</span>
                <span class="driver-class-count">{dg.entries.length}</span>
              </div>
              <ul class="entries">
                {#each dg.entries as b (b.id)}
                  <li class="entry driver-entry" class:restored={b.restored_at}>
                    <div class="entry-glyph" aria-hidden="true">
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></svg>
                    </div>
                    <div class="entry-main">
                      <div class="entry-head">
                        <span class="entry-title">{b.driver_provider ?? $t("view.backups.driver.driverWord")}</span>
                        {#if b.restored_at}
                          <span class="chip chip-success small-chip" title={$t("view.backups.driver.rolledBackHint", { date: fmtDate(b.restored_at) })}>{$t("view.backups.driver.rolledBack")}</span>
                        {:else}
                          <span class="chip chip-update small-chip" title={$t("view.backups.driver.snapshotHint")}>{$t("view.backups.driver.snapshot")}</span>
                        {/if}
                      </div>
                      <div class="entry-meta mono">
                        <span class="file">{b.dll_filename}</span>
                        <span class="sep">·</span>
                        <span>v{b.previous_version ?? "?"}</span>
                        <span class="sep">·</span>
                        <span class="truncate hwid" title={b.hardware_id ?? ""}>{b.hardware_id ?? "—"}</span>
                        <span class="sep">·</span>
                        <span title={b.created_at}>{fmtDate(b.created_at)}</span>
                      </div>
                    </div>
                    <div class="entry-actions">
                      <button
                        class="btn btn-sm btn-ghost"
                        onclick={() => revealBackup(b)}
                        title={$t("view.backups.driver.revealHint")}
                        disabled={openingPath === b.id}
                      >
                        {#if openingPath === b.id}
                          <span class="spin"></span>
                        {:else}
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                        {/if}
                      </button>
                      <button
                        class="btn btn-sm btn-accent"
                        disabled={restoringDriverId === b.id}
                        onclick={() => doDriverRestore(b)}
                        title={$t("view.backups.driver.rollBackHint")}
                      >
                        {#if restoringDriverId === b.id}
                          <span class="spin"></span>
                          {$t("view.backups.driver.rollingBack")}
                        {:else}
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9c-2.52 0-4.85.93-6.63 2.46"/><polyline points="3 4 3 9 8 9"/></svg>
                          {$t("view.backups.driver.rollBack")}
                        {/if}
                      </button>
                    </div>
                  </li>
                {/each}
              </ul>
            </section>
          {/each}
        </div>
      {/if}
    </section>
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

  .backup-hero { margin-bottom: 16px; overflow: hidden; }
  .hero-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 16px 20px;
  }
  .bk-stat { display: flex; align-items: center; gap: 13px; min-width: 0; }
  .bk-stat-badge { width: 42px; height: 42px; border-radius: 13px; }
  .bk-stat-badge svg { display: block; }
  .bk-stat-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .bk-stat-num {
    font-size: var(--fs-2xl);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tighter);
    font-variant-numeric: tabular-nums;
    line-height: 1.15;
  }
  .bk-stat-num.is-update { color: var(--update); }
  .bk-stat-num.is-success { color: var(--success); }
  .bk-stat-lbl {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .hero-meta-strip {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .hero-meta-item { display: inline-flex; align-items: baseline; gap: 6px; }
  .hero-meta-label {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .hero-meta-value {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .hero-meta-value.mono { font-family: var(--font-mono, monospace); font-size: 12px; }
  .hero-meta-sep {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-muted);
    opacity: 0.4;
  }
  .hero-meta-warn { gap: 5px; color: var(--danger); cursor: help; }
  .hero-meta-warn svg { display: block; }
  .hero-meta-warn .hero-meta-value { color: var(--danger); }

  .backup-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .group-by-toggle {
    display: inline-flex;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }
  .group-by-toggle .seg-btn {
    display: inline-flex;
    align-items: center;
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 600;
    background: transparent;
    border: none;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .group-by-toggle .seg-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .group-by-toggle .seg-btn.active { background: var(--accent-dim); color: var(--accent); }
  .group-by-toggle .seg-btn:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
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
    grid-template-columns: 96px 1fr auto;
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
    width: 96px;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-art-fallback);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    position: relative;
  }
  .group-thumb::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(0,0,0,0) 60%, rgba(0,0,0,0.45) 100%);
    pointer-events: none;
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
  .group-stats :global(strong) { color: var(--text-secondary); font-weight: 600; }
  .stat-line.is-update { color: var(--update); }
  .stat-line.is-success { color: var(--success); }
  .stat-line.is-missing { color: var(--danger); }
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
    -webkit-backdrop-filter: blur(8px);
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .bulk-bar { background: var(--bg-elevated); }
  }
  @media (prefers-reduced-transparency: reduce) {
    .bulk-bar { background: var(--bg-elevated); }
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
  .entry.missing .entry-glyph { background: var(--danger-dim); color: var(--danger); opacity: 0.75; }
  .entry.missing .entry-title { color: var(--text-secondary); }
  .entry-meta .is-missing { color: var(--danger); font-weight: 600; }
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

  .driver-section { margin-top: 28px; }
  .driver-head { margin-bottom: 4px; }
  .driver-sub { max-width: 640px; margin-bottom: 14px; }
  .driver-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }
  .driver-group-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-input);
  }
  .driver-class {
    font-size: var(--fs-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-secondary);
  }
  .driver-class-count {
    font-size: var(--fs-2xs);
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    padding: 0 7px;
    font-variant-numeric: tabular-nums;
  }
  .entry-meta .hwid { max-width: 320px; display: inline-block; vertical-align: bottom; }
  .entry.driver-entry { grid-template-columns: 32px 1fr auto; }
</style>
