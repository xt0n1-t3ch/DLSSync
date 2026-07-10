<script lang="ts">
  import { t } from "../lib/i18n/index";
  import { launcherLabel } from "../lib/labels";
  import { favoriteIds, toggleFavorite } from "../lib/stores";
  import type { DetectedGame } from "../lib/api";

  let {
    game,
    coverAccentColor,
    loading,
    rescanning,
    scanError,
    recordCount,
    outdatedCount,
    aheadCount,
    acActive,
    acSeverity,
    acStatus,
    acWarningMessage,
    dlssEnabler,
    managedExternally,
    onClose,
    onLearnMore,
  }: {
    game: DetectedGame;
    coverAccentColor: string | null;
    loading: boolean;
    rescanning: boolean;
    scanError: string | null;
    recordCount: number;
    outdatedCount: number;
    aheadCount: number;
    acActive: boolean;
    acSeverity: "warning" | "danger";
    acStatus: string | null;
    acWarningMessage: string;
    dlssEnabler: boolean;
    managedExternally: boolean;
    onClose: () => void;
    onLearnMore: () => void;
  } = $props();

  let imgErrored = $state(false);
  $effect(() => {
    void game.id;
    imgErrored = false;
  });

  let isFavorite = $derived($favoriteIds.has(game.id));
</script>

<button class="detail-back" onclick={onClose} title={$t("component.gameDrawer.backToLibraryTitle")} aria-label={$t("component.gameDrawer.backToLibrary")}>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
</button>
<button
  class="detail-fav"
  class:is-fav={isFavorite}
  onclick={() => void toggleFavorite(game.id)}
  aria-pressed={isFavorite}
  title={isFavorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
  aria-label={isFavorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
>
  <svg width="16" height="16" viewBox="0 0 24 24" fill={isFavorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
</button>
<header class="detail-hero" data-launcher={game.launcher} style:--game-accent={coverAccentColor ?? "var(--accent)"}>
  <div class="drawer-art">
    {#if game.image_url && !imgErrored}
      <img src={game.image_url} alt={game.name} onerror={() => (imgErrored = true)} />
    {:else}
      <div class="drawer-art-fallback">{game.name.slice(0, 1).toUpperCase()}</div>
    {/if}
    <div class="drawer-art-overlay"></div>
  </div>
  <div class="drawer-meta">
    <span class="launcher-chip">{launcherLabel(game.launcher)}</span>
    <h2 class="drawer-title">{game.name}</h2>
    <p class="drawer-path mono truncate" title={game.install_dir}>{game.install_dir}</p>
  </div>
</header>

<div
  class="status-ribbon"
  class:is-update={!loading && !scanError && outdatedCount > 0}
  class:is-success={!loading && !scanError && recordCount > 0 && outdatedCount === 0}
  class:is-danger={!!scanError}
  class:is-muted={loading || (!scanError && recordCount === 0)}
  aria-live="polite"
>
  {#if loading || rescanning}
    <span class="ribbon-dot is-pulse"></span>
    <span>{$t("component.gameDrawer.ribbon.scanning")}</span>
  {:else if scanError}
    <span class="ribbon-dot"></span>
    <span>{$t("component.gameDrawer.ribbon.scanFailed")} <span class="mono">{scanError}</span></span>
  {:else if recordCount === 0}
    <span class="ribbon-dot"></span>
    <span>{$t("component.gameDrawer.ribbon.noDlls")}</span>
  {:else if outdatedCount === 0}
    <span class="ribbon-dot"></span>
    <span>{$t("component.gameDrawer.ribbon.allUpToDate", { count: recordCount })}</span>
  {:else}
    <span class="ribbon-dot is-pulse"></span>
    <span>{$t("component.gameDrawer.ribbon.updatesReady", { count: outdatedCount })}{aheadCount > 0 ? $t("component.gameDrawer.ribbon.aheadSuffix", { count: aheadCount }) : ""}</span>
  {/if}
</div>

{#if acActive}
  <div class="warning-banner edge-accent" class:is-warning={acSeverity !== "danger"} class:is-danger={acSeverity === "danger"} role="alert">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
    <span class="warning-text">{acWarningMessage}{#if acStatus} {acStatus}{/if}</span>
    <button class="learn-more" title={$t("component.gameDrawer.anticheat.learnMoreTitle")} onclick={onLearnMore}>{$t("component.gameDrawer.anticheat.learnMore")}</button>
  </div>
{/if}

{#if dlssEnabler}
  <div class="warning-banner edge-accent is-info" role="status">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
    <span class="warning-text">{$t("note.enablerManaged")}</span>
  </div>
{/if}

{#if managedExternally}
  <div class="warning-banner edge-accent is-info" role="status" data-testid="nvidia-app-managed">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
    <span class="warning-text">{$t("note.nvidiaAppManaged")}</span>
  </div>
{/if}

<style>
  .detail-back {
    position: absolute;
    top: var(--space-3);
    left: var(--space-3);
    z-index: 5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-full);
    color: var(--art-chrome-fg);
    background: var(--art-chrome-scrim);
    border: 1px solid var(--art-chrome-border);
    cursor: pointer;
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .detail-back:hover { background: var(--art-chrome-scrim-strong); border-color: var(--art-chrome-border-strong); transform: translateX(-1px); }
  .detail-back:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .detail-fav {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    z-index: 5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-full);
    color: var(--art-chrome-fg);
    background: var(--art-chrome-scrim);
    border: 1px solid var(--art-chrome-border);
    cursor: pointer;
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-instant) var(--ease);
  }
  .detail-fav:hover { background: var(--art-chrome-scrim-strong); color: var(--gh-star); border-color: var(--art-chrome-border-strong); }
  .detail-fav:active { transform: scale(0.92); }
  .detail-fav:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .detail-fav.is-fav { color: var(--gh-star); border-color: color-mix(in oklab, var(--gh-star) 55%, transparent); }

  .detail-hero {
    flex-shrink: 0;
    position: relative;
    border: none;
    border-radius: 0;
    overflow: hidden;
  }
  .drawer-art { width: 100%; height: clamp(132px, 20vh, 190px); overflow: hidden; position: relative; }
  .drawer-art::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--game-accent, var(--launcher-accent, var(--accent)));
    z-index: 2;
    pointer-events: none;
  }
  .drawer-art img { width: 100%; height: 100%; object-fit: cover; }
  .drawer-art-fallback {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-art-fallback);
    color: var(--accent);
    font-size: var(--fs-display);
    font-weight: 700;
    opacity: 0.55;
  }
  .drawer-art-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(0, 0, 0, 0.12) 22%,
      rgba(0, 0, 0, 0) 44%,
      rgba(0, 0, 0, 0.58) 72%,
      rgba(0, 0, 0, 0.92) 100%
    );
    pointer-events: none;
  }
  .drawer-meta {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: var(--space-4) var(--space-5) var(--space-4);
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .launcher-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px var(--space-2);
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
    background: var(--accent);
    color: var(--accent-fg);
  }
  .drawer-title {
    font-size: var(--fs-xl-plus);
    font-weight: 700;
    line-height: var(--lh-tight);
    letter-spacing: var(--letter-tighter);
    color: var(--art-chrome-fg);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.6), 0 2px 12px rgba(0, 0, 0, 0.8);
  }
  .drawer-path {
    font-size: var(--fs-xs);
    color: var(--art-chrome-fg-dim);
    max-width: 100%;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.7);
  }

  .status-ribbon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    padding: 11px var(--space-4);
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 0;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    background: var(--bg-card);
    font-variant-numeric: tabular-nums;
  }
  .status-ribbon.is-update { color: var(--update); background: var(--update-dim); }
  .status-ribbon.is-success { color: var(--success); background: var(--success-dim); }
  .status-ribbon.is-danger { color: var(--danger); background: var(--danger-dim); }
  .status-ribbon.is-muted { color: var(--text-muted); }
  .ribbon-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 6px currentColor;
    flex-shrink: 0;
  }
  .ribbon-dot.is-pulse { animation: pulse 2s var(--ease) infinite; }

  .warning-banner {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    column-gap: var(--space-3);
    row-gap: var(--space-2);
    margin: var(--space-3) var(--space-4) 0;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--warning-dim);
    border: 1px solid var(--warning);
    color: var(--warning);
    font-size: var(--fs-sm);
    line-height: var(--lh-snug);
  }
  .warning-banner.is-danger {
    background: var(--danger-dim);
    border-color: var(--danger);
    color: var(--danger);
  }
  .warning-banner.is-info {
    background: var(--info-dim);
    border-color: var(--info);
    color: var(--info);
  }
  .warning-banner svg { flex-shrink: 0; margin-top: 2px; }
  .warning-text {
    flex: 1 1 0;
    min-width: 0;
    overflow-wrap: anywhere;
    white-space: normal;
  }
  .learn-more {
    margin-left: auto;
    height: 28px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-cap);
    color: currentColor;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: var(--letter-wide);
    border: 1px solid currentColor;
    flex: 0 0 auto;
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .learn-more:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .learn-more:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
</style>
