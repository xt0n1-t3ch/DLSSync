<script lang="ts">
  import type { DetectedGame, DllRecord } from "../lib/api";
  import {
    LAUNCHER_ACCENTS,
    launcherLabel,
    recordFeature,
    featureShort,
    featureIconId,
    featureVendor,
    FEATURE_ORDER,
    VENDOR_ACCENTS,
    GROUP_ACCENT,
    type UpdateStatus,
    type FeatureSlot,
  } from "../lib/labels";
  import { gameDlls, gameDllsLoading, gameStatuses, relationContext } from "../lib/stores";
  import { dllRelation } from "../lib/relation";
  import { launcherIcon } from "../lib/launcherIcons";
  import FeatureIcon from "./FeatureIcon.svelte";

  let { game, hidden = false, onApply, onOpenFolder, onBlacklist, onClick }: {
    game: DetectedGame;
    hidden?: boolean;
    onApply: (g: DetectedGame) => void;
    onOpenFolder: (g: DetectedGame) => void;
    onBlacklist: (g: DetectedGame) => void;
    onClick: (g: DetectedGame) => void;
  } = $props();

  function handleCardKey(e: KeyboardEvent): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onClick(game);
    }
  }

  let imgErrored = $state(false);
  let status: UpdateStatus = $derived(($gameStatuses[game.id] ?? "unknown") as UpdateStatus);
  let loading = $derived($gameDllsLoading[game.id] ?? false);
  let dlls: DllRecord[] = $derived($gameDlls[game.id] ?? []);

  function recordOutdated(r: DllRecord): boolean {
    return dllRelation(r, $relationContext) === "outdated";
  }

  let outdatedCount = $derived(dlls.filter(recordOutdated).length);

  type FeatureChip = {
    feature: FeatureSlot;
    outdated: boolean;
    count: number;
    vendorAccent: string;
    short: string;
    iconId: string;
  };

  let featureChips = $derived.by<FeatureChip[]>(() => {
    const map = new Map<FeatureSlot, FeatureChip>();
    for (const r of dlls) {
      const f = recordFeature(r);
      const existing = map.get(f);
      const out = recordOutdated(r);
      if (!existing) {
        map.set(f, {
          feature: f,
          outdated: out,
          count: 1,
          vendorAccent:
            f === "advanced" ? GROUP_ACCENT.advanced : VENDOR_ACCENTS[featureVendor(f)] ?? GROUP_ACCENT.advanced,
          short: f === "advanced" ? "Other" : featureShort(f),
          iconId: featureIconId(f),
        });
      } else {
        existing.count += 1;
        if (out) existing.outdated = true;
      }
    }
    const ordered: FeatureSlot[] = [...FEATURE_ORDER, "advanced"];
    return ordered.map((f) => map.get(f)).filter((v): v is FeatureChip => !!v);
  });

  let visibleChips = $derived(featureChips.slice(0, 4));
  let hiddenChipCount = $derived(Math.max(0, featureChips.length - visibleChips.length));

  let accent = $derived(LAUNCHER_ACCENTS[game.launcher] ?? "#22d3ee");
  let brandMark = $derived(launcherIcon(game.launcher));

  let haloVariant = $derived(
    status === "outdated" ? "is-update" :
    status === "scan_failed" ? "is-danger" :
    "is-neutral"
  );
  let haloActive = $derived(status === "outdated" || status === "scan_failed");
</script>

<div
  class="game-card halo {haloVariant}"
  class:is-active={haloActive}
  class:status-outdated={status === "outdated"}
  class:status-up_to_date={status === "up_to_date"}
  class:status-no_dlls={status === "no_dlls"}
  class:status-scan_failed={status === "scan_failed"}
  class:is-hidden={hidden}
  role="button"
  tabindex="0"
  onclick={() => onClick(game)}
  onkeydown={handleCardKey}
>
  <div class="art" style:--launcher-accent={accent}>
    {#if game.image_url && !imgErrored}
      <img
        class="art-img"
        src={game.image_url}
        alt={game.name}
        loading="lazy"
        onerror={() => (imgErrored = true)}
      />
    {:else}
      <div class="art-fallback" class:is-empty-dotted={status === "no_dlls"}>
        <span class="art-fallback-text">{game.name.slice(0, 1).toUpperCase()}</span>
        {#if status === "no_dlls"}
          <span class="art-empty-hint">Custom folder · DLLs may live elsewhere</span>
        {/if}
      </div>
    {/if}
    <div class="art-overlay"></div>
    <div class="art-top">
      <span class="launcher-chip" style:background={accent} title={launcherLabel(game.launcher)} aria-label={launcherLabel(game.launcher)}>
        <svg class="launcher-mark" viewBox={brandMark.viewBox} fill="currentColor" aria-hidden="true">
          <path d={brandMark.path} />
        </svg>
        <span class="launcher-text">{launcherLabel(game.launcher)}</span>
      </span>
      {#if status === "scan_failed"}
        <span class="status-pill is-danger" title="Scan failed — open and click Rescan" aria-label="Scan failed">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        </span>
      {:else if outdatedCount > 0}
        <span class="status-count" aria-label={`${outdatedCount} update${outdatedCount === 1 ? "" : "s"} available`}>
          {outdatedCount}
        </span>
      {/if}
    </div>
    {#if hidden}
      <div class="hidden-ribbon" aria-label="Hidden">Hidden</div>
    {/if}
    <div class="art-hover-actions" onclick={(e) => e.stopPropagation()} role="presentation">
      {#if hidden}
        <button class="hover-btn primary" onclick={() => onBlacklist(game)} title="Restore to library">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.93 6.63 2.46"/><polyline points="21 4 21 9 16 9"/></svg>
          Restore
        </button>
      {:else if status === "outdated"}
        <button class="hover-btn primary" onclick={() => onApply(game)} title="Apply latest DLLs">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.93 6.63 2.46"/><polyline points="21 4 21 9 16 9"/></svg>
          Apply
        </button>
      {/if}
      <button class="hover-btn round" onclick={() => onOpenFolder(game)} title="Open install folder" aria-label="Open install folder">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
      </button>
      {#if !hidden}
        <button class="hover-btn round" onclick={() => onBlacklist(game)} title="Hide from list" aria-label="Hide from list">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
        </button>
      {/if}
    </div>
  </div>
  <div class="body">
    <h3 class="game-name truncate" title={game.name}>{game.name}</h3>
    {#if loading}
      <div class="loading-row">
        <span class="loading-dot"></span>
        <span class="loading-text">Scanning</span>
      </div>
    {:else if dlls.length === 0}
      <div class="meta-row">
        <span class="meta-text">No supported DLLs</span>
      </div>
    {:else}
      <div class="feature-chips">
        {#each visibleChips as fc}
          <span
            class="feature-chip"
            class:outdated={fc.outdated}
            style:--chip-accent={fc.vendorAccent}
            title="{fc.short}{fc.count > 1 ? ` · ${fc.count} files` : ''}{fc.outdated ? ' · update available' : ''}"
          >
            <span class="feature-chip-icon" aria-hidden="true">
              <FeatureIcon id={fc.iconId} size={12} strokeWidth={1.8} />
            </span>
            <span class="feature-chip-name">{fc.short}</span>
            {#if fc.outdated}
              <span class="feature-chip-pulse" aria-hidden="true"></span>
            {/if}
          </span>
        {/each}
        {#if hiddenChipCount > 0}
          <span class="feature-chip overflow" title="{hiddenChipCount} more">+{hiddenChipCount}</span>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .game-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: transform 0.18s var(--ease), border-color 0.18s var(--ease), box-shadow 0.18s var(--ease);
    cursor: pointer;
    text-align: left;
    width: 100%;
    font: inherit;
    color: inherit;
    background: var(--bg-card);
  }
  .game-card:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .game-card:hover {
    transform: translateY(-2px);
    border-color: var(--accent-ring);
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.55), 0 0 0 1px var(--accent-dim);
  }
  .game-card.is-hidden { opacity: 0.78; }
  .game-card.is-hidden .art-img { filter: grayscale(0.4) brightness(0.85); }
  .hidden-ribbon {
    position: absolute;
    top: 10px;
    right: -22px;
    transform: rotate(35deg);
    background: var(--danger);
    color: #fff;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 3px 28px;
    z-index: 3;
    box-shadow: 0 2px 6px rgba(0,0,0,0.4);
    pointer-events: none;
  }

  .art {
    position: relative;
    aspect-ratio: var(--card-art-aspect);
    background: var(--bg-art-fallback);
    overflow: hidden;
  }
  .art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    transition: transform 0.4s var(--ease-out);
  }
  .game-card:hover .art-img { transform: scale(1.06); }
  .art-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-art-fallback);
    color: var(--text-muted);
  }
  .art-fallback-text {
    font-size: 56px;
    font-weight: 700;
    letter-spacing: -0.04em;
    color: var(--launcher-accent, var(--accent));
    opacity: 0.55;
  }
  .art-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(0,0,0,0.25) 0%, rgba(0,0,0,0) 30%, rgba(0,0,0,0) 60%, rgba(7,9,13,0.85) 100%);
    pointer-events: none;
  }
  :global([data-theme="light"]) .art-overlay {
    background: linear-gradient(180deg, rgba(0,0,0,0.10) 0%, rgba(0,0,0,0) 30%, rgba(0,0,0,0) 70%, rgba(255,255,255,0.55) 100%);
  }
  .art-top {
    position: absolute;
    top: 10px;
    left: 10px;
    right: 10px;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
    z-index: 2;
  }
  .launcher-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px 4px 7px;
    border-radius: var(--radius-full);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
    color: #0a0d13;
    box-shadow: 0 1px 4px rgba(0,0,0,0.35);
  }
  .launcher-mark {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }
  .launcher-text { line-height: 1; }
  .status-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-full);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }
  .status-pill.is-danger {
    background: rgba(239, 68, 68, 0.20);
    color: var(--danger);
    border: 1px solid rgba(239, 68, 68, 0.45);
  }
  .status-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 7px;
    border-radius: var(--radius-full);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: var(--fs-xs);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
  }
  .art-empty-hint {
    position: absolute;
    bottom: 14px;
    left: 14px;
    right: 14px;
    font-size: 10px;
    color: var(--text-muted);
    text-align: center;
    letter-spacing: 0.01em;
  }
  .art-fallback.is-empty-dotted {
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-md);
    margin: 14px;
    width: calc(100% - 28px);
    height: calc(100% - 28px);
    position: absolute;
    inset: 14px;
  }
  .art-hover-actions {
    position: absolute;
    bottom: 10px;
    left: 10px;
    right: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.2s var(--ease-out), transform 0.2s var(--ease-out);
    z-index: 2;
  }
  .game-card:hover .art-hover-actions,
  .game-card:focus-within .art-hover-actions { opacity: 1; transform: translateY(0); }
  .hover-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: var(--radius-md);
    background: rgba(7, 9, 13, 0.78);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    color: #ffffff;
    font-size: 11.5px;
    font-weight: 600;
    border: 1px solid rgba(255, 255, 255, 0.12);
    transition: background 0.15s var(--ease), border-color 0.15s var(--ease);
  }
  .hover-btn:hover { background: rgba(255, 255, 255, 0.14); border-color: rgba(255, 255, 255, 0.22); }
  .hover-btn.round {
    width: 30px;
    height: 30px;
    padding: 0;
    border-radius: 50%;
    justify-content: center;
  }
  :global([data-theme="light"]) .hover-btn {
    background: rgba(255, 255, 255, 0.92);
    color: #0a0c10;
    border-color: rgba(15, 23, 42, 0.18);
    box-shadow: 0 4px 12px rgba(15, 23, 42, 0.14);
  }
  :global([data-theme="light"]) .hover-btn:hover {
    background: #ffffff;
    border-color: rgba(15, 23, 42, 0.35);
  }
  .hover-btn.primary {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
    flex: 1;
    justify-content: center;
  }
  .hover-btn.primary:hover { background: var(--accent-hover); border-color: var(--accent-hover); }

  .body {
    padding: 12px 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .game-name {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .feature-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .feature-chip {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px 3px 6px;
    font-size: 10.5px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    letter-spacing: 0.01em;
  }
  .feature-chip.outdated {
    background: var(--update-dim);
    color: var(--update);
    border-color: rgba(34, 211, 238, 0.30);
  }
  .feature-chip.overflow {
    color: var(--text-muted);
    padding: 3px 8px;
    background: transparent;
  }
  .feature-chip-icon {
    display: inline-flex;
    color: var(--chip-accent, var(--accent));
  }
  .feature-chip.outdated .feature-chip-icon { color: var(--update); }
  .feature-chip-name { line-height: 1; }
  .feature-chip-pulse {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--update);
    box-shadow: 0 0 6px var(--update);
    margin-left: 2px;
    animation: pulseDot 1.6s var(--ease) infinite;
  }
  :global([data-theme="light"]) .feature-chip.outdated {
    background: rgba(8, 145, 178, 0.12);
    color: #075d76;
    border-color: rgba(8, 145, 178, 0.35);
  }

  .loading-row, .meta-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .loading-dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: blink 1.2s infinite;
  }
  @keyframes blink { 0%, 100% { opacity: 0.3 } 50% { opacity: 1 } }
  @keyframes pulseDot {
    0%, 100% { transform: scale(1); opacity: 0.6; }
    50% { transform: scale(1.4); opacity: 1; }
  }
</style>
