<script lang="ts">
  import type { DetectedGame, DllRecord } from "../lib/api";
  import {
    launcherLabel,
    recordFeature,
    featureShort,
    featureIconId,
    featureVendor,
    FEATURE_ORDER,
    vendorAccentVar,
    vendorInkVar,
    GROUP_ACCENT_VAR,
    type UpdateStatus,
    type FeatureSlot,
  } from "../lib/labels";
  import { gameDlls, gameDllsLoading, gameStatuses, relationContext } from "../lib/stores";
  import { dllRelation } from "../lib/relation";
  import { launcherIcon } from "../lib/launcherIcons";
  import { setActiveArt } from "../lib/artContext";
  import { t } from "../lib/i18n/index";
  import FeatureIcon from "./FeatureIcon.svelte";

  let { game, hidden = false, favorite = false, onApply, onOpenFolder, onBlacklist, onClick, onContextMenu, onToggleFavorite }: {
    game: DetectedGame;
    hidden?: boolean;
    favorite?: boolean;
    onApply: (g: DetectedGame) => void;
    onOpenFolder: (g: DetectedGame) => void;
    onBlacklist: (g: DetectedGame) => void;
    onClick: (g: DetectedGame) => void;
    onContextMenu?: (g: DetectedGame, e: MouseEvent) => void;
    onToggleFavorite?: (g: DetectedGame) => void;
  } = $props();

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
    vendorInk: string;
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
            f === "advanced" ? GROUP_ACCENT_VAR.advanced : vendorAccentVar(featureVendor(f)),
          vendorInk: vendorInkVar(featureVendor(f)),
          short: f === "advanced" ? $t("feature.advanced.short") : featureShort(f),
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
  role="presentation"
  oncontextmenu={onContextMenu ? (e) => { e.preventDefault(); onContextMenu(game, e); } : undefined}
  onmouseenter={() => setActiveArt(game.image_url)}
>
  <div class="art" data-launcher={game.launcher}>
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
          <span class="art-empty-hint">{$t("component.card.customFolderHint")}</span>
        {/if}
      </div>
    {/if}
    <div class="art-overlay"></div>
    <div class="art-top">
      <span class="launcher-chip" title={launcherLabel(game.launcher)} aria-label={launcherLabel(game.launcher)}>
        <svg class="launcher-mark" viewBox={brandMark.viewBox} fill="currentColor" aria-hidden="true">
          <path d={brandMark.path} />
        </svg>
        <span class="launcher-text">{launcherLabel(game.launcher)}</span>
      </span>
      <div class="art-top-end">
        {#if onToggleFavorite}
          <button
            class="fav-btn"
            class:is-fav={favorite}
            aria-pressed={favorite}
            aria-label={favorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
            title={favorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
            onclick={(e) => { e.stopPropagation(); onToggleFavorite?.(game); }}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill={favorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
          </button>
        {/if}
        {#if status === "scan_failed"}
          <span class="status-pill is-danger" title={$t("component.card.scanFailedTitle")} aria-label={$t("status.scan_failed")}>
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
          </span>
        {:else if outdatedCount > 0}
          <span class="status-count" aria-label={$t("component.card.updatesAvailable", { count: outdatedCount })}>
            {outdatedCount}
          </span>
        {/if}
      </div>
    </div>
    {#if hidden}
      <div class="hidden-ribbon" aria-label={$t("component.card.hidden")}>{$t("component.card.hidden")}</div>
    {/if}
    <div class="art-hover-actions" onclick={(e) => e.stopPropagation()} role="presentation">
      {#if hidden}
        <button class="hover-btn primary" onclick={() => onBlacklist(game)} title={$t("component.card.restoreToLibrary")}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.93 6.63 2.46"/><polyline points="21 4 21 9 16 9"/></svg>
          {$t("common.restore")}
        </button>
      {:else if status === "outdated"}
        <button class="hover-btn primary" onclick={() => onApply(game)} title={$t("component.card.applyLatest")}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.93 6.63 2.46"/><polyline points="21 4 21 9 16 9"/></svg>
          {$t("common.apply")}
        </button>
      {/if}
      <button class="hover-btn round" onclick={() => onOpenFolder(game)} title={$t("component.card.openInstallFolder")} aria-label={$t("component.card.openInstallFolder")}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
      </button>
      {#if !hidden}
        <button class="hover-btn round" onclick={() => onBlacklist(game)} title={$t("component.card.hideFromList")} aria-label={$t("component.card.hideFromList")}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
        </button>
      {/if}
    </div>
  </div>
  <div class="body">
    <h3 class="game-name truncate" title={game.name}>
      <button
        class="game-name-btn truncate"
        onclick={() => onClick(game)}
        onfocus={() => setActiveArt(game.image_url)}
      >{game.name}</button>
    </h3>
    {#if loading}
      <div class="loading-row">
        <span class="loading-dot"></span>
        <span class="loading-text">{$t("status.scanning")}</span>
      </div>
    {:else if dlls.length === 0}
      <div class="meta-row">
        <span class="meta-text">{$t("component.card.noSupportedDlls")}</span>
      </div>
    {:else}
      <div class="feature-chips">
        {#each visibleChips as fc}
          <span
            class="feature-chip"
            class:outdated={fc.outdated}
            style:--chip-accent={fc.vendorAccent}
            style:--chip-ink={fc.vendorInk}
            title="{fc.short}{fc.count > 1 ? $t('component.card.chipFilesSuffix', { count: fc.count }) : ''}{fc.outdated ? $t('component.card.chipUpdateSuffix') : ''}"
          >
            <span class="feature-chip-icon" aria-hidden="true">
              <FeatureIcon id={fc.iconId} size={12} strokeWidth={1.8} />
            </span>
            <span class="feature-chip-name">{fc.short}</span>
          </span>
        {/each}
        {#if hiddenChipCount > 0}
          <span class="feature-chip overflow" title={$t("component.card.moreCount", { count: hiddenChipCount })}>+{hiddenChipCount}</span>
        {/if}
      </div>
      <div class="card-status">
        {#if status === "outdated"}
          <span class="state-dot" data-state="outdated"></span>
          <span class="card-status-text is-outdated">{$t("component.card.updatesShort", { count: outdatedCount })}</span>
        {:else if status === "scan_failed"}
          <span class="state-dot" data-state="failed"></span>
          <span class="card-status-text is-danger">{$t("status.scan_failed")}</span>
        {:else}
          <span class="state-dot" data-state="current"></span>
          <span class="card-status-text">{$t("status.up_to_date")}</span>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .game-card {
    position: relative;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: var(--card-edge);
    transition: transform 0.18s var(--ease), border-color 0.18s var(--ease), box-shadow 0.18s var(--ease);
    cursor: pointer;
    text-align: left;
    width: 100%;
    font: inherit;
    color: inherit;
    background: var(--bg-card);
  }
  .game-name-btn {
    display: block;
    max-width: 100%;
    padding: 0;
    background: transparent;
    border: none;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-align: left;
    cursor: pointer;
  }
  .game-name-btn::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 1;
  }
  .game-name-btn:focus-visible { outline: none; }
  .game-card:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .game-card:hover {
    transform: translateY(-2px);
    border-color: var(--accent-ring);
    box-shadow: var(--shadow-card-hover);
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
    background:
      radial-gradient(
        120% 120% at 30% 18%,
        color-mix(in oklab, var(--launcher-accent, var(--accent)) 16%, transparent),
        transparent 58%
      ),
      linear-gradient(150deg, var(--bg-elevated), var(--bg-art-fallback));
    color: var(--text-muted);
  }
  .art-fallback-text {
    font-size: 56px;
    font-weight: 800;
    letter-spacing: -0.04em;
    color: var(--launcher-accent, var(--accent));
    opacity: 0.42;
    text-shadow: 0 1px 0 color-mix(in oklab, var(--launcher-accent, var(--accent)) 20%, transparent);
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
  .art-top-end { display: inline-flex; align-items: center; gap: 6px; }
  .fav-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-full);
    color: rgba(255, 255, 255, 0.82);
    background: rgba(7, 9, 13, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.14);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), transform var(--dur-instant) var(--ease);
  }
  .fav-btn:hover { color: var(--gh-star); background: rgba(7, 9, 13, 0.78); }
  .fav-btn:active { transform: scale(0.92); }
  .fav-btn:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .fav-btn.is-fav { color: var(--gh-star); border-color: color-mix(in oklab, var(--gh-star) 46%, transparent); }
  :global([data-theme="light"]) .fav-btn { background: rgba(255, 255, 255, 0.86); color: rgba(15, 23, 42, 0.7); border-color: rgba(15, 23, 42, 0.16); }
  :global([data-theme="light"]) .fav-btn.is-fav { color: #b9860b; border-color: color-mix(in oklab, #b9860b 46%, transparent); }
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
    color: #ffffff;
    background: rgba(7, 9, 13, 0.68);
    border: 1px solid rgba(255, 255, 255, 0.16);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.35);
  }
  .launcher-mark {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    color: var(--launcher-brand, #94a3b8);
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
    color: var(--chip-ink, var(--text-secondary));
    border: 1px solid color-mix(in oklab, var(--chip-accent, var(--accent)) 32%, var(--border));
    letter-spacing: 0.01em;
  }
  .feature-chip.outdated {
    background: var(--chip-ink, var(--accent));
    color: var(--vendor-chip-fg);
    border-color: transparent;
    font-weight: 700;
  }
  .feature-chip.outdated .feature-chip-icon { color: var(--vendor-chip-fg); }
  .feature-chip.overflow {
    color: var(--text-muted);
    padding: 3px 8px;
    background: transparent;
  }
  .feature-chip-icon {
    display: inline-flex;
    color: var(--chip-ink, var(--chip-accent, var(--accent)));
  }
  .feature-chip-name { line-height: 1; }

  .card-status { display: flex; align-items: center; gap: 6px; margin-top: 1px; }
  .card-status-text {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
  }
  .card-status-text.is-outdated { color: var(--warning); }
  .card-status-text.is-danger { color: var(--danger); }

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
</style>
