<script lang="ts">
  import type { DetectedGame, DllRecord } from "../lib/api";
  import {
    LAUNCHER_ACCENTS,
    launcherLabel,
    recordFeature,
    featureShort,
    featureVendor,
    VENDOR_ACCENTS,
    GROUP_ACCENT,
    STATUS_LABELS,
    type UpdateStatus,
  } from "../lib/labels";
  import { gameDlls, gameDllsLoading, gameStatuses, relationContext } from "../lib/stores";
  import { dllRelation, targetVersion } from "../lib/relation";
  import { launcherIcon } from "../lib/launcherIcons";

  let { game, hidden = false, onApply, onOpenFolder, onBlacklist, onClick }: {
    game: DetectedGame;
    hidden?: boolean;
    onApply: (g: DetectedGame) => void;
    onOpenFolder: (g: DetectedGame) => void;
    onBlacklist: (g: DetectedGame) => void;
    onClick: (g: DetectedGame) => void;
  } = $props();

  let status: UpdateStatus = $derived(($gameStatuses[game.id] ?? "unknown") as UpdateStatus);
  let loading = $derived($gameDllsLoading[game.id] ?? false);
  let dlls: DllRecord[] = $derived($gameDlls[game.id] ?? []);
  let imgErrored = $state(false);
  let accent = $derived(LAUNCHER_ACCENTS[game.launcher] ?? "var(--accent)");
  let brandMark = $derived(launcherIcon(game.launcher));

  function isOutdated(r: DllRecord): boolean {
    return dllRelation(r, $relationContext) === "outdated";
  }

  let outdatedDlls = $derived(dlls.filter(isOutdated));
  let primaryOutdated = $derived(outdatedDlls[0] ?? null);
  let primaryTarget = $derived(primaryOutdated ? targetVersion(primaryOutdated, $relationContext) : null);

  let outdatedChips = $derived.by(() => {
    const seen = new Set<string>();
    const chips: { label: string; accent: string }[] = [];
    for (const r of outdatedDlls) {
      const f = recordFeature(r);
      if (seen.has(f)) continue;
      seen.add(f);
      chips.push({
        label: f === "advanced" ? "Other" : featureShort(f),
        accent: f === "advanced" ? GROUP_ACCENT.advanced : VENDOR_ACCENTS[featureVendor(f)] ?? GROUP_ACCENT.advanced,
      });
    }
    return chips;
  });

  let visibleChips = $derived(outdatedChips.slice(0, 4));
  let overflowChips = $derived(Math.max(0, outdatedChips.length - visibleChips.length));

  let haloVariant = $derived(
    status === "outdated" ? "is-update" :
    status === "scan_failed" ? "is-danger" :
    "is-neutral"
  );
  let haloActive = $derived(status === "outdated" || status === "scan_failed");

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onClick(game);
    }
  }
</script>

<div
  class="list-row halo {haloVariant}"
  class:is-active={haloActive}
  class:is-hidden={hidden}
  role="button"
  tabindex="0"
  aria-label={`${game.name}, ${STATUS_LABELS[status] ?? status}`}
  onclick={() => onClick(game)}
  onkeydown={onKey}
>
  <div class="cover" style:--launcher-accent={accent}>
    {#if game.image_url && !imgErrored}
      <img src={game.image_url} alt={game.name} loading="lazy" onerror={() => (imgErrored = true)} />
    {:else}
      <span class="cover-fallback">{game.name.slice(0, 1).toUpperCase()}</span>
    {/if}
  </div>

  <div class="meta">
    <h3 class="row-name truncate" title={game.name}>{game.name}</h3>
    <span class="launcher-chip chip" style:background={accent}>
      <svg viewBox={brandMark.viewBox} width="10" height="10" fill="currentColor" aria-hidden="true">
        <path d={brandMark.path} />
      </svg>
      {launcherLabel(game.launcher)}
    </span>
  </div>

  <div class="status">
    {#if loading}
      <span class="chip chip-neutral">Scanning</span>
    {:else if status === "outdated"}
      <span class="chip chip-update is-strong">{outdatedDlls.length} update{outdatedDlls.length === 1 ? "" : "s"}</span>
    {:else if status === "up_to_date"}
      <span class="chip chip-success">Up to date</span>
    {:else if status === "scan_failed"}
      <span class="chip chip-danger">Scan failed</span>
    {:else if status === "no_dlls"}
      <span class="chip chip-neutral">No DLLs</span>
    {:else}
      <span class="chip chip-neutral">Unknown</span>
    {/if}
  </div>

  <div class="chips">
    {#each visibleChips as c, i (i)}
      <span class="feature-chip" style:--chip-accent={c.accent}>{c.label}</span>
    {/each}
    {#if overflowChips > 0}
      <span class="feature-chip overflow">+{overflowChips}</span>
    {/if}
  </div>

  <div class="version-diff mono" title="Primary version diff">
    {#if primaryOutdated && primaryTarget}
      <span class="ver-old">v{primaryOutdated.current_version ?? "?"}</span>
      <span class="arrow" aria-hidden="true">→</span>
      <span class="ver-new">v{primaryTarget}</span>
    {:else}
      <span class="ver-empty">—</span>
    {/if}
  </div>

  <div class="actions" onclick={(e) => e.stopPropagation()} role="presentation">
    {#if status === "outdated" && !hidden}
      <button class="btn btn-primary btn-sm" onclick={() => onApply(game)} title="Apply latest DLLs">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        Apply
      </button>
    {/if}
    <button class="btn btn-ghost btn-sm" onclick={() => onOpenFolder(game)} title="Open install folder" aria-label="Open install folder">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
    </button>
    <button class="btn btn-ghost btn-sm" onclick={() => onBlacklist(game)} title={hidden ? "Restore to library" : "Hide from list"} aria-label={hidden ? "Restore" : "Hide"}>
      {#if hidden}
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.93 6.63 2.46"/><polyline points="21 4 21 9 16 9"/></svg>
      {:else}
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
      {/if}
    </button>
  </div>
</div>

<style>
  .list-row {
    display: grid;
    grid-template-columns: 72px minmax(180px, 2fr) auto minmax(0, 2fr) auto auto;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
    color: var(--text-primary);
  }
  .list-row:hover { background: var(--bg-card-hover); border-color: var(--border-hover); }
  .list-row:focus-visible { outline: none; border-color: var(--accent); box-shadow: var(--shadow-ring); }
  .list-row.is-hidden { opacity: 0.7; }

  .cover {
    width: 64px;
    height: 36px;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--bg-art-fallback);
    position: relative;
    flex-shrink: 0;
  }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .cover-fallback {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 700;
    color: var(--launcher-accent, var(--accent));
    opacity: 0.7;
  }

  .meta { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .row-name {
    font-size: var(--fs-md);
    font-weight: 600;
    letter-spacing: var(--letter-tight);
  }
  .launcher-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: #0a0d13;
    text-transform: none;
    letter-spacing: 0;
    width: fit-content;
    border-color: transparent;
  }

  .status { min-width: 100px; }

  .chips { display: flex; flex-wrap: nowrap; gap: 4px; overflow: hidden; min-width: 0; }
  .feature-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    font-size: 10.5px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--update-dim);
    color: var(--update);
    border: 1px solid rgba(34, 211, 238, 0.30);
    white-space: nowrap;
  }
  .feature-chip.overflow {
    background: transparent;
    color: var(--text-muted);
    border-color: var(--border);
  }

  .version-diff {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .ver-old { color: var(--text-muted); }
  .ver-new { color: var(--update); font-weight: 600; }
  .arrow { color: var(--text-muted); }
  .ver-empty { color: var(--text-placeholder); }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  @media (max-width: 1000px) {
    .list-row { grid-template-columns: 56px 1fr auto auto; gap: 10px; }
    .chips, .version-diff { display: none; }
  }
</style>
