<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { activeApplies, showToast, games, settings, persistSettings, type ApplyTracker } from "../lib/stores";
  import { fly, fade, slide } from "svelte/transition";
  import { applyUpdate, type ApplyProgress, type DllRecord } from "../lib/api";
  import { get } from "svelte/store";
  import {
    featureTitle,
    featureFromFamily,
    featureIconId,
    filenameFromPath,
    familyLabel,
    familyVendor,
    familyCatalogKey,
    vendorLabel,
    type FeatureSlot,
  } from "../lib/labels";
  import FeatureIcon from "./FeatureIcon.svelte";

  let { onClose }: { onClose: () => void } = $props();

  const STAGES: { id: string; label: string }[] = [
    { id: "download", label: "Download" },
    { id: "verify_sha", label: "Verify SHA" },
    { id: "verify_signature", label: "Verify signature" },
    { id: "backup", label: "Backup current" },
    { id: "replace", label: "Install new" },
    { id: "verify_post", label: "Verify installed" },
    { id: "complete", label: "Done" },
  ];

  type GroupStatus = "running" | "done" | "failed" | "partial";
  interface ApplyGroup {
    key: string;
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
    status: GroupStatus;
  }

  let entries = $derived(Object.values($activeApplies));
  let gameNameById = $derived.by<Map<string, string>>(() => {
    const m = new Map<string, string>();
    for (const g of $games) m.set(g.id, g.name);
    return m;
  });

  let groups = $derived.by<ApplyGroup[]>(() => {
    const by = new Map<string, ApplyGroup>();
    for (const e of entries) {
      const slot: FeatureSlot = featureFromFamily(e.family);
      const key = `${e.game_id}|${slot}|${e.target_version}`;
      let g = by.get(key);
      if (!g) {
        const title = slot === "advanced" ? familyLabel(e.family) : featureTitle(slot);
        g = {
          key,
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
          status: "running",
        };
        by.set(key, g);
      }
      g.items.push(e);
    }
    for (const g of by.values()) {
      g.items.sort((a, b) =>
        filenameFromPath(a.dll_path).localeCompare(filenameFromPath(b.dll_path)),
      );
      for (const it of g.items) {
        if (it.stage === "complete") g.doneCount += 1;
        else if (it.stage === "failed") g.failedCount += 1;
        else g.runningCount += 1;
      }
      if (g.runningCount > 0) g.status = "running";
      else if (g.failedCount === 0) g.status = "done";
      else if (g.doneCount === 0) g.status = "failed";
      else g.status = "partial";
    }
    const arr = Array.from(by.values());
    arr.sort((a, b) => {
      const order = { running: 0, failed: 1, partial: 2, done: 3 } as Record<GroupStatus, number>;
      const d = order[a.status] - order[b.status];
      if (d !== 0) return d;
      return a.gameName.localeCompare(b.gameName);
    });
    return arr;
  });

  let totalGroups = $derived(groups.length);
  let doneGroups = $derived(groups.filter((g) => g.status === "done").length);
  let failedGroups = $derived(groups.filter((g) => g.status === "failed" || g.status === "partial").length);
  let runningGroups = $derived(groups.filter((g) => g.status === "running").length);
  let anyRunning = $derived(runningGroups > 0);
  let allDone = $derived(totalGroups > 0 && runningGroups === 0);

  let totalItems = $derived(entries.length);
  let doneItems = $derived(entries.filter((e) => e.stage === "complete").length);
  let failedItems = $derived(entries.filter((e) => e.stage === "failed").length);
  let progressPct = $derived(totalItems === 0 ? 0 : Math.round(((doneItems + failedItems) / totalItems) * 100));

  let expanded = $state<Record<string, boolean>>({});
  let retryingId = $state<string | null>(null);

  function toggle(key: string): void {
    expanded = { ...expanded, [key]: !expanded[key] };
  }

  let unlisten: (() => void) | undefined;

  onMount(async () => {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<ApplyProgress>("apply_progress", (event) => {
      const p = event.payload;
      activeApplies.update((map) => {
        const existing = map[p.apply_id];
        if (!existing) return map;
        const failed_at =
          p.stage === "failed" ? (existing.failed_at_stage ?? existing.stage) : existing.failed_at_stage;
        const next: ApplyTracker = {
          ...existing,
          stage: p.stage,
          failed_at_stage: failed_at,
          message: p.message,
          progress: p.progress,
          error: p.error,
        };
        return { ...map, [p.apply_id]: next };
      });
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  function stageStatus(entry: ApplyTracker, stageId: string): "pending" | "running" | "ok" | "fail" {
    const order = STAGES.map((s) => s.id);
    const stageIdx = order.indexOf(stageId);
    if (entry.stage === "failed") {
      const failedIdx = entry.failed_at_stage ? order.indexOf(entry.failed_at_stage) : -1;
      if (failedIdx >= 0) {
        if (stageIdx < failedIdx) return "ok";
        if (stageIdx === failedIdx) return "fail";
        return "pending";
      }
      return "pending";
    }
    const currentIdx = order.indexOf(entry.stage);
    if (stageIdx < currentIdx) return "ok";
    if (stageIdx === currentIdx) return entry.stage === "complete" ? "ok" : "running";
    return "pending";
  }

  function dismiss(): void {
    activeApplies.set({});
    onClose();
  }

  async function retryOne(entry: ApplyTracker): Promise<void> {
    if (retryingId) return;
    retryingId = entry.apply_id;
    activeApplies.update((m) => {
      const cur = m[entry.apply_id];
      if (!cur) return m;
      return {
        ...m,
        [entry.apply_id]: {
          ...cur,
          stage: "download",
          failed_at_stage: null,
          message: "Retrying…",
          error: null,
          progress: 0,
        },
      };
    });
    try {
      await applyUpdate({
        apply_id: entry.apply_id,
        game_id: entry.game_id,
        game_label: entry.game_label,
        dll_path: entry.dll_path,
        vendor: familyVendor(entry.family as DllRecord["family"]),
        family: familyCatalogKey(entry.family as DllRecord["family"]),
        target_version: entry.target_version,
      });
    } catch (err: unknown) {
      const msg =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err);
      activeApplies.update((m) => {
        const cur = m[entry.apply_id];
        if (!cur) return m;
        return {
          ...m,
          [entry.apply_id]: {
            ...cur,
            stage: "failed",
            failed_at_stage: cur.failed_at_stage ?? cur.stage,
            error: msg,
            message: msg,
          },
        };
      });
    } finally {
      retryingId = null;
    }
  }

  async function retryGroup(g: ApplyGroup): Promise<void> {
    const failed = g.items.filter((it) => it.stage === "failed");
    for (const it of failed) {
      await retryOne(it);
    }
  }

  function isSignatureError(err: string | null | undefined): boolean {
    if (!err) return false;
    const lower = err.toLowerCase();
    return (
      lower.includes("crypt_e_no_match") ||
      lower.includes("allow unsigned dlls") ||
      lower.includes("authenticode signature could not be read") ||
      lower.includes("no authenticode subject")
    );
  }

  async function allowUnsignedAndRetryGroup(g: ApplyGroup): Promise<void> {
    const current = get(settings);
    if (!current) {
      showToast("danger", "Settings not loaded — cannot toggle");
      return;
    }
    if (!current.advanced.allow_unsigned_dlls) {
      const next = {
        ...current,
        advanced: { ...current.advanced, allow_unsigned_dlls: true },
      };
      try {
        await persistSettings(next);
        showToast("info", "Unsigned DLL bypass enabled — retrying");
      } catch (err: unknown) {
        showToast("danger", `Could not enable unsigned bypass: ${String(err)}`);
        return;
      }
    }
    await retryGroup(g);
  }

  function copyError(err: string | null): void {
    if (!err) return;
    navigator.clipboard?.writeText(err).then(
      () => showToast("success", "Error copied"),
      () => showToast("warning", "Clipboard unavailable"),
    );
  }

  function statusLabel(g: ApplyGroup): string {
    if (g.status === "done") return "Updated";
    if (g.status === "failed") return "Failed";
    if (g.status === "partial") return `${g.doneCount}/${g.items.length} updated`;
    return `${g.doneCount}/${g.items.length} updated`;
  }

  function firstError(g: ApplyGroup): string | null {
    const f = g.items.find((it) => it.stage === "failed");
    return f?.error ?? null;
  }
</script>

<div class="backdrop" transition:fade={{ duration: 150 }} role="presentation" onclick={() => allDone && dismiss()} onkeydown={(e) => { if (e.key === "Escape" && allDone) dismiss(); }} tabindex="-1"></div>
<div class="modal" transition:fly={{ y: 20, duration: 200 }} role="dialog" aria-labelledby="apply-modal-title">
  <div class="modal-aurora"></div>
  <header class="modal-head">
    <div class="head-left">
      <span class="head-eyebrow">{allDone ? (failedGroups > 0 ? "Apply completed with failures" : "Apply complete") : "Applying updates"}</span>
      <h2 id="apply-modal-title" class="head-title">
        {#if allDone}
          {#if failedGroups === 0}
            All {totalGroups} feature{totalGroups === 1 ? "" : "s"} updated
          {:else}
            {doneGroups} ok · {failedGroups} failed · {totalGroups} total
          {/if}
        {:else}
          {doneGroups} ok · {failedGroups} failed · {runningGroups} running · {totalGroups} total
        {/if}
      </h2>
      <div class="head-summary">
        {#if failedGroups > 0}
          <span class="chip chip-danger small-chip">{failedGroups} failed</span>
        {/if}
        {#if doneGroups > 0}
          <span class="chip chip-success small-chip">{doneGroups} ok</span>
        {/if}
        {#if runningGroups > 0}
          <span class="chip chip-update small-chip is-strong">{runningGroups} in flight</span>
        {/if}
        <span class="head-files-summary">{doneItems}/{totalItems} file{totalItems === 1 ? "" : "s"}</span>
      </div>
    </div>
    {#if allDone}
      <button class="btn btn-primary" onclick={dismiss}>Close</button>
    {/if}
  </header>

  <div class="progress-track" role="progressbar" aria-valuenow={progressPct} aria-valuemin="0" aria-valuemax="100">
    <div class="progress-fill" style:width="{progressPct}%" class:is-danger={failedGroups > 0 && !anyRunning} class:is-success={allDone && failedGroups === 0}></div>
  </div>

  <div class="group-list">
    {#each groups as g (g.key)}
      <article class="apply-group" class:done={g.status === "done"} class:failed={g.status === "failed"} class:partial={g.status === "partial"}>
        <button class="group-head" onclick={() => toggle(g.key)} aria-expanded={!!expanded[g.key]}>
          <div class="group-glyph" aria-hidden="true">
            <FeatureIcon id={g.iconId} size={20} />
          </div>
          <div class="group-meta">
            <div class="group-title-row">
              <span class="group-title">{g.featureTitleText}</span>
              <span class="group-arrow">→</span>
              <span class="group-version mono">v{g.targetVersion}</span>
              <span class="group-vendor">{vendorLabel(g.vendorKey)}</span>
            </div>
            <div class="group-meta-row">
              <span class="group-game truncate">{g.gameName}</span>
              <span class="meta-dot"></span>
              <span class="group-filecount">{g.items.length} file{g.items.length === 1 ? "" : "s"}</span>
            </div>
          </div>
          <div class="group-right">
            {#if g.status === "done"}
              <span class="status-pill is-success">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                Updated
              </span>
            {:else if g.status === "failed"}
              <span class="status-pill is-danger">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>
                Failed
              </span>
            {:else if g.status === "partial"}
              <span class="status-pill is-warning">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                {statusLabel(g)}
              </span>
            {:else}
              <span class="status-pill is-running">
                <span class="spinner-tiny"></span>
                {g.doneCount}/{g.items.length}
              </span>
            {/if}
            <svg class="group-chev" class:open={expanded[g.key]} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
          </div>
        </button>

        {#if (g.status === "failed" || g.status === "partial") && !expanded[g.key]}
          {@const err = firstError(g)}
          {#if err}
            {@const isSig = isSignatureError(err)}
            <div class="group-error-preview">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              <span class="group-error-preview-text truncate">{isSig ? "Vendor ships this DLL unsigned. Click below to enable bypass and retry." : err.split("\n")[0]}</span>
              {#if isSig}
                <button class="btn btn-sm btn-accent" onclick={(e) => { e.stopPropagation(); void allowUnsignedAndRetryGroup(g); }} title="Enable Allow unsigned DLLs in Settings → Advanced and retry this group">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M9 12l2 2 4-4"/><circle cx="12" cy="12" r="10"/></svg>
                  Allow unsigned & retry
                </button>
              {:else}
                <button class="btn btn-sm btn-accent" onclick={(e) => { e.stopPropagation(); void retryGroup(g); }}>
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"/></svg>
                  Retry
                </button>
              {/if}
            </div>
          {/if}
        {/if}

        {#if expanded[g.key]}
          <div class="group-body" transition:slide={{ duration: 180 }}>
            {#each g.items as e (e.apply_id)}
              {@const fname = filenameFromPath(e.dll_path)}
              {@const isFailed = e.stage === "failed"}
              {@const isDone = e.stage === "complete"}
              <div class="file-row" class:failed={isFailed} class:done={isDone}>
                <div class="file-head">
                  <span class="file-name mono">{fname}</span>
                  <span class="file-status">
                    {#if isDone}
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                      Done
                    {:else if isFailed}
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>
                      Failed
                    {:else}
                      <span class="spinner-tiny"></span>
                      {e.message}
                    {/if}
                  </span>
                </div>
                <ol class="stages-compact">
                  {#each STAGES as s, i}
                    {@const status = stageStatus(e, s.id)}
                    <li class="stage stage-{status}" title={s.label}>
                      <span class="stage-icon">
                        {#if status === "ok"}
                          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                        {:else if status === "fail"}
                          <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                        {:else if status === "running"}
                          <span class="dot-pulse"></span>
                        {:else}
                          <span class="dot"></span>
                        {/if}
                      </span>
                      {#if i < STAGES.length - 1}<span class="stage-line" class:filled={status === "ok"}></span>{/if}
                    </li>
                  {/each}
                </ol>
                {#if isFailed && e.error}
                  <div class="file-error">
                    <pre class="file-error-msg">{e.error}</pre>
                    <div class="file-error-actions">
                      <button class="btn btn-sm btn-ghost" onclick={() => copyError(e.error)} title="Copy error">
                        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                      </button>
                      <button class="btn btn-sm btn-accent" onclick={() => retryOne(e)} disabled={retryingId === e.apply_id}>
                        {#if retryingId === e.apply_id}
                          <span class="spinner-tiny"></span>
                          Retrying
                        {:else}
                          Retry
                        {/if}
                      </button>
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </article>
    {/each}
  </div>

  <footer class="modal-foot">
    {#if anyRunning}
      <p class="modal-foot-text">Do not close DLSSync while updates are running. Failures auto-rollback.</p>
    {:else if failedGroups > 0}
      <p class="modal-foot-text">{failedGroups} feature{failedGroups === 1 ? "" : "s"} failed — click a row to inspect, or Retry directly.</p>
    {:else}
      <p class="modal-foot-text">All features updated. Restart any running games for changes to take effect.</p>
    {/if}
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.76);
    backdrop-filter: blur(4px);
    z-index: 200;
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(820px, 94vw);
    max-height: 88vh;
    background: var(--bg-card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    z-index: 201;
    overflow: hidden;
    isolation: isolate;
  }
  .modal-aurora { display: none; }

  .modal-head {
    padding: 22px 26px 14px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 14px;
    align-items: end;
  }
  .head-left { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .head-eyebrow {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--accent);
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
  }
  .head-title {
    font-size: var(--fs-xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .head-summary { display: inline-flex; gap: 8px; margin-top: 6px; flex-wrap: wrap; align-items: center; }
  .small-chip { padding: 2px 8px; font-size: var(--fs-2xs); letter-spacing: 0.04em; }
  .head-files-summary { font-size: var(--fs-2xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .progress-track {
    height: 4px;
    background: var(--bg-elevated);
    margin: 0 26px;
    border-radius: var(--radius-full);
    overflow: hidden;
    position: relative;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-full);
    transition: width 0.4s var(--ease-out), background 0.2s var(--ease);
  }
  .progress-fill.is-success { background: var(--success); }
  .progress-fill.is-danger { background: var(--danger); }

  .group-list {
    padding: 16px 26px 18px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .apply-group {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition: border-color 0.2s var(--ease);
    flex-shrink: 0;
  }
  .apply-group > .group-head { border-radius: var(--radius-lg) var(--radius-lg) 0 0; }
  .apply-group > .group-head:only-child { border-radius: var(--radius-lg); }
  .apply-group.done {
    border-color: rgba(52, 211, 153, 0.30);
    background: var(--success-dim);
  }
  .apply-group.failed { border-color: rgba(239, 68, 68, 0.45); }
  .apply-group.partial { border-color: rgba(245, 185, 80, 0.45); }

  .group-head {
    width: 100%;
    display: grid;
    grid-template-columns: 44px 1fr auto;
    gap: 16px;
    align-items: center;
    padding: 16px 20px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    transition: background 0.12s var(--ease);
    min-height: 78px;
  }
  .group-head:hover { background: var(--bg-card-hover); }
  .group-glyph {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .apply-group.done .group-glyph { background: var(--success-dim); color: var(--success); }
  .apply-group.failed .group-glyph { background: var(--danger-dim); color: var(--danger); }
  .apply-group.partial .group-glyph { background: var(--warning-dim); color: var(--warning); }
  .group-meta { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .group-title-row { display: inline-flex; align-items: center; gap: 9px; flex-wrap: wrap; }
  .group-title { font-size: var(--fs-md); font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .group-arrow { color: var(--text-muted); font-weight: 400; }
  .group-version {
    color: var(--update);
    font-weight: 700;
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
  }
  .group-vendor {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 700;
    color: var(--text-muted);
    padding: 1px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
  }
  .group-meta-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    min-width: 0;
    margin-top: 2px;
  }
  .group-game { color: var(--text-secondary); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 360px; }
  .meta-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--text-muted); flex-shrink: 0; }
  .group-filecount { font-variant-numeric: tabular-nums; color: var(--text-muted); }

  .group-right { display: inline-flex; align-items: center; gap: 10px; flex-shrink: 0; }
  .group-chev { color: var(--text-muted); transition: transform 0.2s var(--ease); }
  .group-chev.open { transform: rotate(90deg); color: var(--accent); }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: var(--letter-wide);
    text-transform: uppercase;
  }
  .status-pill.is-success { background: var(--success-dim); color: var(--success); }
  .status-pill.is-danger { background: var(--danger-dim); color: var(--danger); }
  .status-pill.is-warning { background: var(--warning-dim); color: var(--warning); }
  .status-pill.is-running { background: var(--accent-dim); color: var(--accent); text-transform: none; letter-spacing: 0; font-variant-numeric: tabular-nums; }

  .group-error-preview {
    display: grid;
    grid-template-columns: 14px 1fr auto;
    gap: 10px;
    align-items: center;
    padding: 10px 18px;
    background: var(--danger-dim);
    color: var(--danger);
    border-top: 1px solid rgba(239, 68, 68, 0.25);
  }
  .group-error-preview-text {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .group-body {
    border-top: 1px solid var(--border);
    background: var(--bg-card);
    padding: 12px 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .file-row {
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .file-row.done { border-color: rgba(52, 211, 153, 0.3); }
  .file-row.failed { border-color: rgba(239, 68, 68, 0.4); }
  .file-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }
  .file-name { font-size: var(--fs-xs); color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0; }
  .file-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    color: var(--text-muted);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-row.done .file-status { color: var(--success); }
  .file-row.failed .file-status { color: var(--danger); }

  .stages-compact {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    align-items: center;
  }
  .stage {
    display: flex;
    align-items: center;
    position: relative;
    justify-content: flex-start;
  }
  .stage-icon {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--bg-card);
    border: 1px solid var(--border);
    flex-shrink: 0;
    z-index: 2;
  }
  .stage-pending .dot { width: 4px; height: 4px; background: var(--text-muted); border-radius: 50%; opacity: 0.4; }
  .stage-running .stage-icon { background: var(--accent-dim); border-color: var(--accent); }
  .stage-running .dot-pulse { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); animation: pulseDot 1s var(--ease) infinite; }
  .stage-ok .stage-icon { background: var(--success); border-color: var(--success); color: var(--accent-fg); }
  .stage-fail .stage-icon { background: var(--danger); border-color: var(--danger); color: #fff; }
  .stage-line {
    flex: 1;
    height: 1px;
    background: var(--border);
    margin-left: -1px;
    margin-right: -1px;
    z-index: 1;
  }
  .stage-line.filled { background: var(--success); }

  .file-error {
    margin-top: 10px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    padding: 9px 11px;
    background: var(--danger-dim);
    border: 1px solid rgba(239, 68, 68, 0.35);
    border-radius: var(--radius-sm);
    color: var(--danger);
  }
  .file-error-msg {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.4;
  }
  .file-error-actions { display: inline-flex; gap: 5px; align-items: flex-start; flex-shrink: 0; }

  .modal-foot {
    padding: 12px 26px;
    border-top: 1px solid var(--border);
    background: var(--bg-input);
  }
  .modal-foot-text { font-size: var(--fs-xs); color: var(--text-muted); text-align: center; }

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
  @keyframes pulseDot {
    0% { transform: scale(0.8); opacity: 0.6; }
    50% { transform: scale(1.15); opacity: 1; }
    100% { transform: scale(0.8); opacity: 0.6; }
  }
</style>
