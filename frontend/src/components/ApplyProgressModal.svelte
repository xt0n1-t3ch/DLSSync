<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fly, fade, slide } from "svelte/transition";
  import { get } from "svelte/store";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    activeApplies,
    downloadProgressByGroup,
    games,
    settings,
    persistSettings,
    showToast,
    currentView,
    type ApplyTracker,
    type GroupDownloadState,
  } from "../lib/stores";
  import {
    APPLY_STAGES,
    TRAY_SHOW_PROGRESS_EVENT,
    buildIssueReport,
    type ApplyErrorClass,
    type DllRecord,
  } from "../lib/api";
  import {
    featureTitle,
    featureFromFamily,
    featureIconId,
    filenameFromPath,
    familyLabel,
    familyVendor,
    vendorLabel,
    type FeatureSlot,
  } from "../lib/labels";
  import FeatureIcon from "./FeatureIcon.svelte";
  import {
    classifyApplyError,
    ERROR_CLASS_LABEL,
    ERROR_CLASS_TONE,
  } from "../lib/applyErrorClass";
  import {
    cancelAll,
    cancelOne,
    retryFailedTrackers,
    retrySingleApply,
  } from "../lib/applyController";
  import {
    formatBytes,
    formatElapsedSince,
    formatEta,
    formatSpeed,
    percentOf,
  } from "../lib/formatHuman";

  let { onClose }: { onClose: () => void } = $props();

  type GroupStatus = "running" | "done" | "failed" | "partial";
  type FilterMode = "all" | "running" | "failed" | "done";

  interface ApplyGroup {
    key: string;
    group_id: string;
    gameId: string;
    gameName: string;
    featureSlot: FeatureSlot;
    featureTitleText: string;
    iconId: string;
    targetVersion: string;
    vendorKey: string;
    items: ApplyTracker[];
    doneCount: number;
    failedCount: number;
    runningCount: number;
    cancelledCount: number;
    status: GroupStatus;
    download: GroupDownloadState | null;
    primaryErrorClass: ApplyErrorClass | null;
  }

  interface DedupedError {
    message: string;
    class: ApplyErrorClass;
    affected: ApplyTracker[];
  }

  const entries = $derived(Object.values($activeApplies));
  const gameNameById = $derived.by<Map<string, string>>(() => {
    const m = new Map<string, string>();
    for (const g of $games) m.set(g.id, g.name);
    return m;
  });

  let now = $state<number>(Date.now());
  let tick: ReturnType<typeof setInterval> | undefined;

  let filter = $state<FilterMode>("all");
  let selectedKey = $state<string | null>(null);
  let expanded = $state<Record<string, boolean>>({});
  let retryingId = $state<string | null>(null);
  let busy = $state<boolean>(false);
  let autoFailedApplied = $state(false);

  const groups = $derived.by<ApplyGroup[]>(() => {
    const by = new Map<string, ApplyGroup>();
    for (const e of entries) {
      const slot: FeatureSlot = featureFromFamily(e.family);
      const key = `${e.game_id}|${slot}|${e.target_version}`;
      let g = by.get(key);
      if (!g) {
        const title = slot === "advanced" ? familyLabel(e.family) : featureTitle(slot);
        g = {
          key,
          group_id: e.group_id,
          gameId: e.game_id,
          gameName: gameNameById.get(e.game_id) ?? e.game_id,
          featureSlot: slot,
          featureTitleText: title,
          iconId: featureIconId(slot),
          targetVersion: e.target_version,
          vendorKey: familyVendor(e.family as DllRecord["family"]),
          items: [],
          doneCount: 0,
          failedCount: 0,
          runningCount: 0,
          cancelledCount: 0,
          status: "running",
          download: null,
          primaryErrorClass: null,
        };
        by.set(key, g);
      }
      g.items.push(e);
      if (!g.group_id && e.group_id) g.group_id = e.group_id;
    }
    const dlMap = $downloadProgressByGroup;
    for (const g of by.values()) {
      g.items.sort((a, b) =>
        filenameFromPath(a.dll_path).localeCompare(filenameFromPath(b.dll_path)),
      );
      for (const it of g.items) {
        if (it.stage === "complete") g.doneCount += 1;
        else if (it.stage === "failed") g.failedCount += 1;
        else if (it.stage === "cancelled") g.cancelledCount += 1;
        else g.runningCount += 1;
      }
      if (g.runningCount > 0) g.status = "running";
      else if (g.failedCount === 0 && g.cancelledCount === 0) g.status = "done";
      else if (g.doneCount === 0) g.status = "failed";
      else g.status = "partial";
      if (g.group_id) g.download = dlMap[g.group_id] ?? null;
      const firstFailed = g.items.find((i) => i.stage === "failed" || i.stage === "cancelled");
      g.primaryErrorClass = firstFailed
        ? (firstFailed.error_class as ApplyErrorClass | null) ?? classifyApplyError(firstFailed.error).kind
        : null;
    }
    const arr = Array.from(by.values());
    const order: Record<GroupStatus, number> = { running: 0, failed: 1, partial: 2, done: 3 };
    arr.sort((a, b) => {
      const d = order[a.status] - order[b.status];
      if (d !== 0) return d;
      return a.gameName.localeCompare(b.gameName);
    });
    return arr;
  });

  const filteredGroups = $derived(
    filter === "all"
      ? groups
      : groups.filter((g) => {
          if (filter === "running") return g.status === "running";
          if (filter === "failed") return g.status === "failed" || g.status === "partial";
          if (filter === "done") return g.status === "done";
          return true;
        }),
  );

  const totalGroups = $derived(groups.length);
  const doneGroups = $derived(groups.filter((g) => g.status === "done").length);
  const failedGroups = $derived(
    groups.filter((g) => g.status === "failed" || g.status === "partial").length,
  );
  const runningGroups = $derived(groups.filter((g) => g.status === "running").length);
  const anyRunning = $derived(runningGroups > 0);
  const allDone = $derived(totalGroups > 0 && runningGroups === 0);

  const totalItems = $derived(entries.length);
  const doneItems = $derived(entries.filter((e) => e.stage === "complete").length);
  const failedItems = $derived(
    entries.filter((e) => e.stage === "failed" || e.stage === "cancelled").length,
  );
  const itemProgressPct = $derived(
    totalItems === 0 ? 0 : Math.round(((doneItems + failedItems) / totalItems) * 100),
  );

  const totalBytesDownloaded = $derived(
    Object.values($downloadProgressByGroup).reduce((acc, d) => acc + d.bytes_downloaded, 0),
  );
  const totalBytesTotal = $derived.by(() => {
    let sum = 0;
    let known = false;
    for (const d of Object.values($downloadProgressByGroup)) {
      if (d.bytes_total) {
        sum += d.bytes_total;
        known = true;
      }
    }
    return known ? sum : null;
  });
  const aggregateSpeed = $derived(
    Object.values($downloadProgressByGroup).reduce((acc, d) => acc + d.bytes_per_sec, 0),
  );
  const downloadPct = $derived(percentOf(totalBytesDownloaded, totalBytesTotal));

  const startedAt = $derived(
    entries.length === 0 ? Date.now() : Math.min(...entries.map((e) => e.started_at)),
  );
  const endedAt = $derived.by<number | null>(() => {
    if (!allDone) return null;
    let max = 0;
    for (const e of entries) {
      if (e.ended_at && e.ended_at > max) max = e.ended_at;
    }
    return max || null;
  });
  const elapsedDisplay = $derived(
    formatElapsedSince(startedAt, allDone ? endedAt ?? now : now),
  );

  const failedSignatureCount = $derived(
    entries.filter((e) => e.stage === "failed" && classifyApplyError(e.error).kind === "signature").length,
  );

  let unlistenTray: UnlistenFn | undefined;
  onMount(async () => {
    tick = setInterval(() => (now = Date.now()), 500);
    unlistenTray = await listen<void>(TRAY_SHOW_PROGRESS_EVENT, () => {
      // Modal is already mounted when this fires; no-op (parent controls visibility).
    });
    if (groups.length > 0 && !selectedKey) {
      const firstFailed = groups.find((g) => g.status === "failed" || g.status === "partial");
      selectedKey = (firstFailed ?? groups[0]).key;
      expanded = { ...expanded, [selectedKey]: true };
    }
  });

  onDestroy(() => {
    if (tick) clearInterval(tick);
    if (unlistenTray) unlistenTray();
  });

  $effect(() => {
    if (!selectedKey && groups.length > 0) {
      const firstFailed = groups.find((g) => g.status === "failed" || g.status === "partial");
      selectedKey = (firstFailed ?? groups[0]).key;
      expanded = { ...expanded, [selectedKey]: true };
    }
    if (selectedKey && !groups.find((g) => g.key === selectedKey)) {
      selectedKey = groups[0]?.key ?? null;
    }
  });

  $effect(() => {
    if (!autoFailedApplied && failedGroups > 0 && filter === "all") {
      autoFailedApplied = true;
      filter = "failed";
    }
  });

  let uniqueGameCount = $derived.by(() => {
    const set = new Set<string>();
    for (const g of groups) {
      if (g.status === "done") set.add(g.gameId);
    }
    return set.size;
  });

  function goToBackups(): void {
    currentView.set("backups");
    void dismiss();
  }

  const selectedGroup = $derived(groups.find((g) => g.key === selectedKey) ?? null);
  const selectedDedupedErrors = $derived.by<DedupedError[]>(() => {
    if (!selectedGroup) return [];
    return dedupeErrors(selectedGroup.items);
  });

  function dedupeErrors(items: ApplyTracker[]): DedupedError[] {
    const byMessage = new Map<string, DedupedError>();
    for (const it of items) {
      if (it.stage !== "failed" && it.stage !== "cancelled") continue;
      const message = (it.error ?? it.message ?? "Unknown error").trim();
      const klass: ApplyErrorClass =
        (it.error_class as ApplyErrorClass | null) ?? classifyApplyError(message).kind;
      const existing = byMessage.get(message);
      if (existing) existing.affected.push(it);
      else byMessage.set(message, { message, class: klass, affected: [it] });
    }
    return Array.from(byMessage.values()).sort((a, b) => b.affected.length - a.affected.length);
  }

  function toggleExpand(key: string): void {
    expanded = { ...expanded, [key]: !expanded[key] };
  }

  function selectGroup(key: string): void {
    selectedKey = key;
    expanded = { ...expanded, [key]: true };
  }

  async function dismiss(): Promise<void> {
    activeApplies.set({});
    downloadProgressByGroup.set({});
    onClose();
  }

  async function handleRetryGroup(g: ApplyGroup): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await retryFailedTrackers(g.items);
    } finally {
      busy = false;
    }
  }

  async function handleRetrySingle(item: ApplyTracker): Promise<void> {
    if (retryingId) return;
    retryingId = item.apply_id;
    try {
      await retrySingleApply(item);
    } finally {
      retryingId = null;
    }
  }

  async function handleRetryAllFailed(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await retryFailedTrackers(entries);
    } finally {
      busy = false;
    }
  }

  async function handleAllowUnsignedAndRetry(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const current = get(settings);
      if (current && !current.advanced.allow_unsigned_dlls) {
        const next = {
          ...current,
          advanced: { ...current.advanced, allow_unsigned_dlls: true },
        };
        try {
          await persistSettings(next);
          showToast("info", "Unsigned DLL bypass enabled");
        } catch (err: unknown) {
          showToast("danger", `Could not enable unsigned bypass: ${String(err)}`);
          return;
        }
      }
      const targets = entries.filter(
        (e) =>
          (e.stage === "failed" || e.stage === "cancelled") &&
          classifyApplyError(e.error).kind === "signature",
      );
      if (targets.length === 0) {
        showToast("info", "No signature failures to retry");
        return;
      }
      await retryFailedTrackers(targets);
    } finally {
      busy = false;
    }
  }

  async function handleCancelAll(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await cancelAll();
    } finally {
      busy = false;
    }
  }

  async function handleCancelOne(item: ApplyTracker): Promise<void> {
    await cancelOne(item.apply_id);
  }

  function copyError(message: string | null): void {
    if (!message) return;
    void navigator.clipboard?.writeText(message).then(
      () => showToast("success", "Error copied"),
      () => showToast("warning", "Clipboard unavailable"),
    );
  }

  function copyReport(): void {
    const report = buildReport();
    void navigator.clipboard?.writeText(report).then(
      () => showToast("success", "Report copied"),
      () => showToast("warning", "Clipboard unavailable"),
    );
  }

  let reportingIssue = $state(false);
  async function reportIssue(): Promise<void> {
    if (reportingIssue) return;
    reportingIssue = true;
    try {
      const report = await buildIssueReport(buildReport());
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(report.url);
    } catch (err: unknown) {
      showToast("danger", `Could not open issue report: ${String(err)}`);
    } finally {
      reportingIssue = false;
    }
  }

  function buildReport(): string {
    const lines: string[] = [];
    lines.push(`DLSSync Apply Report`);
    lines.push(`Generated: ${new Date().toISOString()}`);
    lines.push(`Elapsed: ${elapsedDisplay}`);
    lines.push(
      `Summary: ${doneGroups} done · ${failedGroups} failed · ${runningGroups} running · ${totalGroups} total`,
    );
    lines.push(``);
    for (const g of groups) {
      lines.push(`[${g.status.toUpperCase()}] ${g.gameName} → ${g.featureTitleText} v${g.targetVersion}`);
      lines.push(`  vendor: ${g.vendorKey}  group_id: ${g.group_id || "?"}`);
      if (g.download) {
        lines.push(
          `  download: ${formatBytes(g.download.bytes_downloaded)} / ${formatBytes(g.download.bytes_total)} @ ${formatSpeed(g.download.bytes_per_sec)} (attempt ${g.download.attempt})`,
        );
      }
      for (const it of g.items) {
        const fname = filenameFromPath(it.dll_path);
        const stage = it.stage;
        const elapsed = formatElapsedSince(it.started_at, it.ended_at);
        lines.push(`  - ${fname.padEnd(28)}  [${stage}] ${elapsed}`);
        if (it.error) {
          lines.push(`    error_class: ${it.error_class ?? classifyApplyError(it.error).kind}`);
          for (const errLine of it.error.split("\n")) lines.push(`    ${errLine}`);
        }
      }
      lines.push(``);
    }
    return lines.join("\n");
  }

  function statusLabel(g: ApplyGroup): string {
    if (g.status === "done") return "Updated";
    if (g.status === "failed") return "Failed";
    if (g.status === "partial") return `${g.doneCount}/${g.items.length} updated`;
    return `${g.doneCount}/${g.items.length}`;
  }

  function currentStageLabel(entry: ApplyTracker): string {
    if (entry.stage === "complete") return "Done";
    if (entry.stage === "cancelled") return "Cancelled";
    if (entry.stage === "failed") {
      const failed = APPLY_STAGES.find((s) => s.id === entry.failed_at_stage);
      return failed ? `Failed at ${failed.label.toLowerCase()}` : "Failed";
    }
    const cur = APPLY_STAGES.find((s) => s.id === entry.stage);
    return cur ? `${cur.label}…` : entry.stage;
  }

  function stageStepIndex(entry: ApplyTracker): number {
    if (entry.stage === "complete") return APPLY_STAGES.length;
    if (entry.stage === "failed" || entry.stage === "cancelled") {
      const idx = APPLY_STAGES.findIndex((s) => s.id === entry.failed_at_stage);
      return idx >= 0 ? idx : 0;
    }
    const idx = APPLY_STAGES.findIndex((s) => s.id === entry.stage);
    return idx >= 0 ? idx : 0;
  }

  function headlineText(): string {
    if (allDone) return failedGroups > 0 ? `${doneGroups} of ${totalGroups} updated` : `${totalGroups} feature${totalGroups === 1 ? "" : "s"} updated`;
    if (runningGroups > 0) return `Updating ${runningGroups} of ${totalGroups}`;
    return `${totalGroups} feature${totalGroups === 1 ? "" : "s"} queued`;
  }

  function groupElapsed(g: ApplyGroup): string {
    let start = Infinity;
    let end = 0;
    let running = false;
    for (const it of g.items) {
      if (it.started_at < start) start = it.started_at;
      if (it.stage !== "complete" && it.stage !== "failed" && it.stage !== "cancelled") running = true;
      if (it.ended_at && it.ended_at > end) end = it.ended_at;
    }
    if (start === Infinity) return "—";
    return formatElapsedSince(start, running ? now : end || now);
  }

  function groupProgressPct(g: ApplyGroup): number {
    if (g.items.length === 0) return 0;
    return Math.round(((g.doneCount + g.failedCount + g.cancelledCount) / g.items.length) * 100);
  }
</script>

<div
  class="backdrop"
  transition:fade={{ duration: 150 }}
  role="presentation"
  onclick={() => allDone && dismiss()}
  onkeydown={(e) => {
    if (e.key === "Escape" && allDone) void dismiss();
  }}
  tabindex="-1"
></div>
<div class="modal glass-dialog" transition:fly={{ y: 20, duration: 200 }} role="dialog" aria-labelledby="apply-modal-title">
  <header class="modal-head">
    <div class="head-left">
      <div class="head-eyebrow-row">
        <span class="head-eyebrow">
          {#if allDone}
            {failedGroups > 0 ? "Apply completed with failures" : "Apply complete"}
          {:else}
            Applying updates
          {/if}
        </span>
        <span class="head-elapsed mono">{elapsedDisplay}</span>
      </div>
      <h2 id="apply-modal-title" class="head-title">{headlineText()}</h2>
      <div class="head-stats">
        {#if doneGroups > 0}
          <span class="stat-chip stat-success"><span class="stat-num">{doneGroups}</span><span class="stat-word">done</span></span>
        {/if}
        {#if runningGroups > 0}
          <span class="stat-chip stat-running"><span class="stat-num">{runningGroups}</span><span class="stat-word">in flight</span></span>
        {/if}
        {#if failedGroups > 0}
          <span class="stat-chip stat-failed"><span class="stat-num">{failedGroups}</span><span class="stat-word">failed</span></span>
        {/if}
        <span class="stat-chip stat-total"><span class="stat-num">{totalGroups}</span><span class="stat-word">total</span></span>
        <span class="head-files-summary mono">
          {doneItems}/{totalItems} file{totalItems === 1 ? "" : "s"}
          {#if totalBytesTotal !== null && totalBytesTotal > 0}
            · {formatBytes(totalBytesDownloaded)} of {formatBytes(totalBytesTotal)}
          {/if}
          {#if anyRunning && aggregateSpeed > 0}
            · {formatSpeed(aggregateSpeed)}
          {/if}
        </span>
      </div>
    </div>
    <button
      class="dialog-close"
      onclick={() => allDone && dismiss()}
      disabled={!allDone}
      title={allDone ? "Close" : "Cancel running applies first"}
      aria-label="Close"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
  </header>

  <div class="progress-track" role="progressbar" aria-valuenow={anyRunning ? Math.round(downloadPct) : itemProgressPct} aria-valuemin="0" aria-valuemax="100">
    <div
      class="progress-fill"
      style:width="{anyRunning && totalBytesTotal !== null ? downloadPct : itemProgressPct}%"
      class:is-danger={failedGroups > 0 && !anyRunning}
      class:is-success={allDone && failedGroups === 0}
    ></div>
  </div>

  <main class="modal-body">
    <aside class="rail">
      <div class="filter-row">
        {#each [{ id: "all", label: `All ${totalGroups}` }, { id: "failed", label: `Failed ${failedGroups}` }, { id: "running", label: `Running ${runningGroups}` }, { id: "done", label: `Done ${doneGroups}` }] as f (f.id)}
          <button
            class="filter-chip"
            class:active={filter === f.id}
            onclick={() => (filter = f.id as FilterMode)}
          >
            {f.label}
          </button>
        {/each}
      </div>

      <div class="group-list">
        {#each filteredGroups as g (g.key)}
          <button
            class="group-tile"
            class:done={g.status === "done"}
            class:failed={g.status === "failed"}
            class:partial={g.status === "partial"}
            class:running={g.status === "running"}
            class:selected={selectedKey === g.key}
            onclick={() => selectGroup(g.key)}
          >
            <div class="tile-glyph" aria-hidden="true">
              <FeatureIcon id={g.iconId} size={18} />
            </div>
            <div class="tile-body">
              <div class="tile-title-row">
                <span class="tile-title truncate">{g.featureTitleText}</span>
                <span class="tile-version mono">v{g.targetVersion}</span>
              </div>
              <div class="tile-sub truncate">{g.gameName}</div>
              {#if g.status === "running" && g.download}
                <div class="tile-download" transition:slide={{ duration: 120 }}>
                  <div class="tile-progress">
                    <div
                      class="tile-progress-fill"
                      style:width="{percentOf(g.download.bytes_downloaded, g.download.bytes_total)}%"
                    ></div>
                  </div>
                  <div class="tile-progress-text mono">
                    {formatBytes(g.download.bytes_downloaded)}
                    {#if g.download.bytes_total}/{formatBytes(g.download.bytes_total)}{/if}
                    · {formatSpeed(g.download.bytes_per_sec)}
                    {#if g.download.bytes_total}· ETA {formatEta(g.download.bytes_downloaded, g.download.bytes_total, g.download.bytes_per_sec)}{/if}
                  </div>
                </div>
              {/if}
              {#if g.status !== "running" && g.primaryErrorClass}
                <div class="tile-error-chip" data-tone={ERROR_CLASS_TONE[g.primaryErrorClass]}>
                  {ERROR_CLASS_LABEL[g.primaryErrorClass]}
                </div>
              {/if}
            </div>
            <div class="tile-status">
              {#if g.status === "done"}
                <span class="status-pill is-success">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                  Updated
                </span>
              {:else if g.status === "failed"}
                <span class="status-pill is-danger">Failed</span>
              {:else if g.status === "partial"}
                <span class="status-pill is-warning">{statusLabel(g)}</span>
              {:else}
                <span class="status-pill is-running"><span class="spinner-tiny"></span>{g.doneCount}/{g.items.length}</span>
              {/if}
            </div>
          </button>
        {/each}
        {#if filteredGroups.length === 0}
          <p class="rail-empty">No items match this filter.</p>
        {/if}
      </div>
    </aside>

    <section class="pane">
      {#if selectedGroup}
        {@const g = selectedGroup}
        <header class="pane-head">
          <div class="pane-head-glyph"><FeatureIcon id={g.iconId} size={22} /></div>
          <div class="pane-head-text">
            <h3 class="pane-head-title">
              {g.featureTitleText}
              <span class="pane-head-arrow">→</span>
              <span class="pane-head-version mono">v{g.targetVersion}</span>
            </h3>
            <p class="pane-head-sub">
              <span class="pane-head-game truncate">{g.gameName}</span>
              <span class="dot"></span>
              <span class="vendor-pill">{vendorLabel(g.vendorKey)}</span>
              <span class="dot"></span>
              <span>{g.items.length} file{g.items.length === 1 ? "" : "s"}</span>
            </p>
          </div>
          <button
            class="detail-toggle"
            onclick={() => toggleExpand(g.key)}
            title="Toggle per-file stage detail"
            aria-expanded={!!expanded[g.key]}
          >
            <svg class="detail-chevron" class:open={expanded[g.key]} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
            {expanded[g.key] ? "Hide detail" : "Show detail"}
          </button>
        </header>

        <div class="pane-head-stats">
          <div class="phs-chips">
            {#if g.doneCount > 0}
              <span class="phs-chip phs-done"><span class="phs-num">{g.doneCount}</span> done</span>
            {/if}
            {#if g.runningCount > 0}
              <span class="phs-chip phs-running"><span class="phs-num">{g.runningCount}</span> running</span>
            {/if}
            {#if g.failedCount > 0}
              <span class="phs-chip phs-failed"><span class="phs-num">{g.failedCount}</span> failed</span>
            {/if}
            {#if g.cancelledCount > 0}
              <span class="phs-chip phs-muted"><span class="phs-num">{g.cancelledCount}</span> cancelled</span>
            {/if}
          </div>
          <span class="phs-time mono" title="Elapsed time for this feature">
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
            {groupElapsed(g)}
          </span>
        </div>
        <div class="phs-progress" aria-hidden="true">
          <div
            class="phs-progress-fill"
            class:is-danger={g.status === "failed" || g.status === "partial"}
            class:is-success={g.status === "done"}
            style:width="{groupProgressPct(g)}%"
          ></div>
        </div>

        {#if g.download && g.status === "running"}
          <div class="group-download-block">
            <div class="block-label">
              <span>Downloading shared archive</span>
              <span class="mono attempt">attempt {g.download.attempt}</span>
            </div>
            <div class="block-progress">
              <div
                class="block-progress-fill"
                style:width="{percentOf(g.download.bytes_downloaded, g.download.bytes_total)}%"
              ></div>
            </div>
            <div class="block-stats mono">
              <span>{formatBytes(g.download.bytes_downloaded)}{#if g.download.bytes_total} of {formatBytes(g.download.bytes_total)}{/if}</span>
              <span>{formatSpeed(g.download.bytes_per_sec)}</span>
              {#if g.download.bytes_total}
                <span>ETA {formatEta(g.download.bytes_downloaded, g.download.bytes_total, g.download.bytes_per_sec)}</span>
              {/if}
            </div>
          </div>
        {/if}

        {#if selectedDedupedErrors.length > 0}
          <div class="error-block-list">
            {#each selectedDedupedErrors as de}
              {@const klass = classifyApplyError(de.message)}
              <div class="error-block" data-tone={ERROR_CLASS_TONE[de.class]}>
                <div class="error-block-head">
                  <span class="error-block-kind">{ERROR_CLASS_LABEL[de.class]}</span>
                  <span class="error-block-affected">× {de.affected.length} file{de.affected.length === 1 ? "" : "s"}</span>
                  <button class="btn btn-ghost btn-xs" onclick={() => copyError(de.message)} title="Copy this error">Copy</button>
                </div>
                <pre class="error-block-msg">{de.message}</pre>
                <p class="error-block-hint">{klass.hint}</p>
                <div class="error-block-actions">
                  {#if klass.action === "allow_unsigned_and_retry"}
                    <button class="btn btn-accent btn-sm" disabled={busy} onclick={handleAllowUnsignedAndRetry}>Allow unsigned &amp; retry</button>
                  {:else if klass.action === "refresh_catalog"}
                    <button class="btn btn-accent btn-sm" disabled={busy} onclick={() => handleRetryGroup(g)}>Retry this group</button>
                  {:else if klass.action === "close_game_and_retry"}
                    <button class="btn btn-accent btn-sm" disabled={busy} onclick={() => handleRetryGroup(g)}>Retry (after closing)</button>
                  {:else if klass.action === "report"}
                    <button class="btn btn-ghost btn-sm" onclick={copyReport}>Copy report</button>
                  {:else if klass.action === "elevate"}
                    <button class="btn btn-ghost btn-sm" onclick={() => copyError(de.message)}>Copy error</button>
                  {:else if klass.action === "retry"}
                    <button class="btn btn-accent btn-sm" disabled={busy} onclick={() => handleRetryGroup(g)}>Retry</button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div class="file-list">
          {#each g.items as e (e.apply_id)}
            {@const fname = filenameFromPath(e.dll_path)}
            {@const isFailed = e.stage === "failed" || e.stage === "cancelled"}
            {@const isDone = e.stage === "complete"}
            {@const isRunning = !isFailed && !isDone}
            <article class="file-row" class:failed={isFailed} class:done={isDone}>
              <div class="file-head">
                <span class="file-name mono">{fname}</span>
                <span class="file-status">
                  {#if isDone}
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                    Done · {formatElapsedSince(e.started_at, e.ended_at)}
                  {:else if e.stage === "cancelled"}
                    Cancelled
                  {:else if isFailed}
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>
                    Failed · {formatElapsedSince(e.started_at, e.ended_at)}
                  {:else}
                    <span class="spinner-tiny"></span>
                    {e.message}
                  {/if}
                </span>
              </div>
              {#if expanded[g.key]}
                <div class="file-stage" class:done={isDone} class:failed={isFailed} transition:slide={{ duration: 140 }}>
                  <span class="file-stage-label">{currentStageLabel(e)}</span>
                  {#if !isDone && !isFailed}
                    <span class="file-stage-step mono">step {stageStepIndex(e) + 1} of {APPLY_STAGES.length}</span>
                  {/if}
                  <div class="file-stage-bar" aria-hidden="true">
                    <div class="file-stage-bar-fill" style:width="{(stageStepIndex(e) / APPLY_STAGES.length) * 100}%"></div>
                  </div>
                </div>
              {/if}
              <div class="file-actions">
                {#if isRunning}
                  <button class="btn btn-ghost btn-xs" onclick={() => handleCancelOne(e)}>Cancel</button>
                {/if}
                {#if isFailed}
                  <button class="btn btn-ghost btn-xs" disabled={!e.error} onclick={() => copyError(e.error)}>Copy error</button>
                  <button class="btn btn-accent btn-xs" disabled={retryingId === e.apply_id} onclick={() => handleRetrySingle(e)}>
                    {#if retryingId === e.apply_id}<span class="spinner-tiny"></span>Retrying{:else}Retry{/if}
                  </button>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {:else}
        <div class="pane-empty">
          <h3>No group selected</h3>
          <p>Pick a group on the left to see per-file stages and errors.</p>
        </div>
      {/if}
    </section>
  </main>

  <footer class="action-bar">
    <div class="action-info">
      {#if anyRunning}
        <span>Updates running — closing DLSSync now will cancel them.</span>
      {:else if failedGroups > 0}
        <span>{failedGroups} feature{failedGroups === 1 ? "" : "s"} failed — categorized below.</span>
      {:else if allDone}
        <span>Applied {doneItems} update{doneItems === 1 ? "" : "s"} across {uniqueGameCount} game{uniqueGameCount === 1 ? "" : "s"} · auto-backup created.</span>
      {/if}
    </div>
    <div class="action-cta">
      {#if anyRunning}
        <button class="aura-pill aura-pill-ghost" disabled={busy} onclick={handleCancelAll}>Cancel all</button>
      {/if}
      {#if failedSignatureCount > 0}
        <button class="aura-pill aura-pill-ghost" disabled={busy} onclick={handleAllowUnsignedAndRetry}>
          Allow unsigned &amp; retry ({failedSignatureCount})
        </button>
      {/if}
      {#if failedGroups > 0 && !anyRunning}
        <button class="aura-pill aura-pill-primary" disabled={busy} onclick={handleRetryAllFailed}>Retry all failed</button>
      {/if}
      {#if failedGroups > 0 || allDone}
        <button class="aura-pill aura-pill-ghost" onclick={copyReport}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          Copy report
        </button>
      {/if}
      {#if failedGroups > 0 && !anyRunning}
        <button class="aura-pill aura-pill-ghost" onclick={reportIssue} disabled={reportingIssue} title="Open a pre-filled GitHub issue with this report, your app version, OS, and recent logs">
          {#if reportingIssue}
            <span class="spinner-tiny"></span>
            Preparing
          {:else}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m8 2 1.88 1.88"/><path d="M14.12 3.88 16 2"/><path d="M9 7.13v-1a3.003 3.003 0 1 1 6 0v1"/><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6"/><path d="M12 20v-9"/><path d="M6.53 9C4.6 8.8 3 7.1 3 5"/><path d="M6 13H2"/><path d="M3 21c0-2.1 1.7-3.9 3.8-4"/><path d="M20.97 5c0 2.1-1.6 3.8-3.5 4"/><path d="M22 13h-4"/><path d="M17.2 17c2.1.1 3.8 1.9 3.8 4"/></svg>
            Report issue
          {/if}
        </button>
      {/if}
      {#if allDone && failedGroups === 0 && uniqueGameCount > 0}
        <button class="aura-pill aura-pill-ghost" onclick={goToBackups} title="See the snapshots just created">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
          View backups
        </button>
      {/if}
      {#if allDone}
        <button class="aura-pill aura-pill-primary" onclick={dismiss}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          Dismiss
        </button>
      {/if}
    </div>
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    z-index: 200;
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: clamp(680px, 60vw, 920px);
    max-width: 96vw;
    max-height: 92vh;
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    z-index: 201;
    isolation: isolate;
  }
  .modal-head {
    padding: 24px 56px 16px 28px;
    display: grid;
    grid-template-columns: 1fr;
    gap: 14px;
    align-items: start;
  }
  .head-left { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .head-eyebrow-row { display: inline-flex; gap: 12px; align-items: baseline; }
  .head-eyebrow {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--accent);
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
  }
  .head-elapsed {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .head-title {
    font-size: var(--fs-2xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
    line-height: 1.15;
    margin: 0;
  }
  .head-stats { display: inline-flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-top: 2px; }
  .stat-chip {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    font-variant-numeric: tabular-nums;
    line-height: 1.15;
  }
  .stat-chip .stat-num { font-size: var(--fs-md); font-weight: 700; color: var(--text-primary); }
  .stat-chip .stat-word { font-size: var(--fs-2xs); font-weight: 600; text-transform: uppercase; letter-spacing: var(--letter-wider); color: var(--text-muted); }
  .stat-chip.stat-success { background: var(--badge-green-bg); }
  .stat-chip.stat-success .stat-num { color: var(--badge-green-fg); }
  .stat-chip.stat-success .stat-word { color: var(--badge-green-fg); opacity: 0.75; }
  .stat-chip.stat-running { background: var(--badge-blue-bg); }
  .stat-chip.stat-running .stat-num { color: var(--badge-blue-fg); }
  .stat-chip.stat-running .stat-word { color: var(--badge-blue-fg); opacity: 0.75; }
  .stat-chip.stat-failed { background: var(--badge-red-bg); }
  .stat-chip.stat-failed .stat-num { color: var(--badge-red-fg); }
  .stat-chip.stat-failed .stat-word { color: var(--badge-red-fg); opacity: 0.75; }
  .head-files-summary { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; margin-left: 4px; }
  .dialog-close:disabled { opacity: 0.35; cursor: not-allowed; }
  .dialog-close:disabled:hover { background: transparent; color: var(--text-muted); }

  .progress-track {
    height: 3px;
    background: var(--bg-elevated);
    margin: 0 28px;
    border-radius: var(--radius-full);
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-full);
    transition: width 0.4s var(--ease-out), background 0.2s var(--ease);
  }
  .progress-fill.is-success { background: var(--success); }
  .progress-fill.is-danger { background: var(--danger); }

  .modal-body {
    display: grid;
    grid-template-columns: minmax(280px, 320px) 1fr;
    gap: 0;
    min-height: 0;
    overflow: hidden;
  }
  .rail {
    border-right: 1px solid var(--border);
    background: transparent;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .filter-row {
    display: flex;
    gap: 6px;
    padding: 14px 16px 10px;
    flex-wrap: wrap;
  }
  .filter-chip {
    background: transparent;
    color: var(--text-muted);
    padding: 6px 12px;
    border-radius: var(--radius-full);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .filter-chip:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .filter-chip.active {
    background: var(--accent);
    color: var(--accent-fg);
    box-shadow: 0 2px 8px var(--accent-dim);
  }

  .group-list {
    overflow-y: auto;
    flex: 1;
    padding: 4px 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .group-tile {
    display: grid;
    grid-template-columns: 36px 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 12px 14px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .group-tile:hover { background: var(--bg-card-hover); }
  .group-tile.selected { background: var(--accent-soft); border-color: var(--accent-dim); }
  .tile-glyph {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    background: var(--badge-blue-bg);
    color: var(--badge-blue-fg);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .group-tile.done .tile-glyph { background: var(--badge-green-bg); color: var(--badge-green-fg); }
  .group-tile.failed .tile-glyph { background: var(--badge-red-bg); color: var(--badge-red-fg); }
  .group-tile.partial .tile-glyph { background: var(--badge-orange-bg); color: var(--badge-orange-fg); }
  .tile-body { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .tile-title-row { display: inline-flex; align-items: baseline; gap: 8px; }
  .tile-title { font-size: var(--fs-sm); font-weight: 700; letter-spacing: var(--letter-tight); }
  .tile-version { color: var(--update); font-weight: 700; font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .tile-sub { font-size: var(--fs-xs); color: var(--text-secondary); }
  .tile-download { display: flex; flex-direction: column; gap: 4px; margin-top: 4px; }
  .tile-progress { height: 3px; background: var(--bg-input); border-radius: var(--radius-full); overflow: hidden; }
  .tile-progress-fill { height: 100%; background: var(--accent); border-radius: var(--radius-full); transition: width 0.3s var(--ease-out); }
  .tile-progress-text { font-size: 10px; color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .tile-error-chip {
    margin-top: 4px;
    align-self: flex-start;
    font-size: var(--fs-2xs);
    font-weight: 700;
    padding: 1px 7px;
    border-radius: var(--radius-full);
    letter-spacing: var(--letter-wide);
    text-transform: uppercase;
  }
  .tile-error-chip[data-tone="danger"] { background: var(--danger-dim); color: var(--danger); }
  .tile-error-chip[data-tone="warning"] { background: var(--warning-dim); color: var(--warning); }
  .tile-error-chip[data-tone="info"] { background: var(--info-dim); color: var(--info); }
  .tile-error-chip[data-tone="neutral"] { background: var(--neutral-dim); color: var(--text-muted); }
  .tile-status { display: inline-flex; align-items: center; flex-shrink: 0; }
  .rail-empty { color: var(--text-muted); padding: 18px; text-align: center; font-size: var(--fs-xs); }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 600;
    letter-spacing: var(--letter-wide);
    text-transform: uppercase;
    font-variant-numeric: tabular-nums;
  }
  .status-pill.is-success { background: var(--badge-green-bg); color: var(--badge-green-fg); }
  .status-pill.is-danger  { background: var(--badge-red-bg);   color: var(--badge-red-fg);   }
  .status-pill.is-warning { background: var(--badge-orange-bg); color: var(--badge-orange-fg); }
  .status-pill.is-running { background: var(--badge-blue-bg);  color: var(--badge-blue-fg);  }

  .pane {
    overflow-y: auto;
    padding: 22px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
  }
  .pane-head { display: grid; grid-template-columns: 40px minmax(0, 1fr) auto; gap: 14px; align-items: start; }
  .pane-head-glyph {
    width: 40px; height: 40px;
    background: var(--badge-blue-bg);
    color: var(--badge-blue-fg);
    border-radius: 12px;
    display: inline-flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .pane-head-text { min-width: 0; display: flex; flex-direction: column; gap: 5px; padding-top: 1px; }
  .pane-head-title { font-size: var(--fs-lg); font-weight: 700; margin: 0; display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; letter-spacing: var(--letter-tight); line-height: 1.25; }
  .pane-head-arrow { color: var(--text-muted); font-weight: 400; }
  .pane-head-version { color: var(--update); font-variant-numeric: tabular-nums; }
  .pane-head-sub { display: flex; gap: 8px; color: var(--text-secondary); font-size: var(--fs-sm); margin: 0; align-items: center; flex-wrap: wrap; row-gap: 5px; min-width: 0; }
  .pane-head-game { font-weight: 500; max-width: 100%; }
  .vendor-pill {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 700;
    color: var(--text-secondary);
    padding: 3px 10px;
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
  }
  .dot { width: 3px; height: 3px; border-radius: 50%; background: var(--text-muted); flex-shrink: 0; }

  .detail-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    align-self: start;
    white-space: nowrap;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .detail-toggle:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .detail-toggle:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .detail-chevron { transition: transform var(--dur-fast) var(--ease); }
  .detail-chevron.open { transform: rotate(180deg); }

  .pane-head-stats {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .phs-chips { display: inline-flex; gap: 8px; flex-wrap: wrap; }
  .phs-chip {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    padding: 4px 11px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    color: var(--text-muted);
  }
  .phs-chip .phs-num { font-size: var(--fs-sm); font-weight: 700; font-variant-numeric: tabular-nums; }
  .phs-chip.phs-done { background: var(--badge-green-bg); color: var(--badge-green-fg); }
  .phs-chip.phs-running { background: var(--badge-blue-bg); color: var(--badge-blue-fg); }
  .phs-chip.phs-failed { background: var(--badge-red-bg); color: var(--badge-red-fg); }
  .phs-time {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .phs-progress {
    height: 4px;
    background: var(--bg-elevated);
    border-radius: var(--radius-full);
    overflow: hidden;
    margin-top: -6px;
  }
  .phs-progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-full);
    transition: width var(--dur-normal) var(--ease-out), background var(--dur-fast) var(--ease);
  }
  .phs-progress-fill.is-success { background: var(--success); }
  .phs-progress-fill.is-danger { background: var(--danger); }

  .action-cta .aura-pill:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-cta .aura-pill :global(svg) { flex-shrink: 0; }

  .group-download-block {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .block-label { display: flex; justify-content: space-between; align-items: baseline; font-size: var(--fs-sm); color: var(--text-secondary); font-weight: 600; }
  .attempt { color: var(--text-muted); font-size: var(--fs-2xs); }
  .block-progress { height: 6px; background: var(--bg-card); border-radius: var(--radius-full); overflow: hidden; }
  .block-progress-fill { height: 100%; background: var(--accent); border-radius: var(--radius-full); transition: width 0.4s var(--ease-out); }
  .block-stats { display: inline-flex; gap: 14px; font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .error-block-list { display: flex; flex-direction: column; gap: 10px; }
  .error-block {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    background: var(--bg-input);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .error-block[data-tone="danger"] { border-color: rgba(239, 68, 68, 0.4); background: var(--danger-dim); }
  .error-block[data-tone="warning"] { border-color: rgba(245, 185, 80, 0.4); background: var(--warning-dim); }
  .error-block[data-tone="info"] { border-color: rgba(124, 185, 232, 0.4); background: var(--info-dim); }
  .error-block-head { display: inline-flex; gap: 10px; align-items: center; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: var(--letter-wider); }
  .error-block-kind { color: var(--text-primary); }
  .error-block-affected { color: var(--text-muted); font-weight: 600; }
  .error-block-msg {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .error-block-hint { font-size: var(--fs-xs); color: var(--text-secondary); margin: 0; }
  .error-block-actions { display: inline-flex; gap: 6px; margin-top: 2px; }

  .file-list { display: flex; flex-direction: column; gap: 10px; }
  .file-row {
    padding: 14px 16px;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    display: flex; flex-direction: column; gap: 10px;
  }
  .file-row.done { background: var(--badge-green-bg); }
  .file-row.failed { background: var(--badge-red-bg); }
  .file-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .file-name { font-size: var(--fs-xs); color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0; }
  .file-status { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-2xs); font-weight: 700; text-transform: uppercase; letter-spacing: var(--letter-wide); color: var(--text-muted); max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .file-row.done .file-status { color: var(--badge-green-fg); }
  .file-row.failed .file-status { color: var(--badge-red-fg); }

  .file-stage {
    display: grid;
    grid-template-columns: 1fr auto;
    column-gap: 12px;
    row-gap: 6px;
    align-items: center;
  }
  .file-stage-label {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    font-weight: 500;
    letter-spacing: var(--letter-tight);
  }
  .file-stage.done .file-stage-label { color: var(--success); font-weight: 600; }
  .file-stage.failed .file-stage-label { color: var(--danger); font-weight: 600; }
  .file-stage-step {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .file-stage-bar {
    grid-column: 1 / -1;
    height: 4px;
    background: var(--bg-card);
    border-radius: var(--radius-full);
    overflow: hidden;
  }
  .file-stage-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-full);
    transition: width var(--dur-normal) var(--ease-out);
  }
  .file-stage.done .file-stage-bar-fill { background: var(--success); width: 100% !important; }
  .file-stage.failed .file-stage-bar-fill { background: var(--danger); }

  .file-actions { display: inline-flex; gap: 6px; justify-content: flex-end; }
  .pane-empty { padding: 36px; text-align: center; color: var(--text-muted); }
  .pane-empty h3 { color: var(--text-primary); margin: 0 0 6px; font-size: var(--fs-md); }

  .action-bar {
    padding: 16px 28px;
    border-top: 1px solid var(--border);
    background: transparent;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 12px;
    align-items: center;
  }
  .action-info { font-size: var(--fs-xs); color: var(--text-muted); }
  .action-cta { display: inline-flex; gap: 8px; align-items: center; }
  .btn-xs { padding: 3px 8px; font-size: var(--fs-2xs); }

  .spinner-tiny {
    width: 11px;
    height: 11px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    display: inline-block;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-width: 880px) {
    .modal-body { grid-template-columns: 1fr; }
    .rail { border-right: none; border-bottom: 1px solid var(--border); max-height: 240px; }
  }
</style>
