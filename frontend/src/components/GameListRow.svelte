<script lang="ts">
  import type { DetectedGame, DllRecord } from "../lib/api";
  import {
    launcherLabel,
    recordFeature,
    featureShort,
    featureVendor,
    type UpdateStatus,
  } from "../lib/labels";
  import { gameDlls, gameDllsLoading, gameStatuses, relationContext } from "../lib/stores";
  import { dllRelation, targetVersion } from "../lib/relation";
  import { launcherIcon } from "../lib/launcherIcons";
  import { t } from "../lib/i18n/index";

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

  let status: UpdateStatus = $derived(($gameStatuses[game.id] ?? "unknown") as UpdateStatus);
  let loading = $derived($gameDllsLoading[game.id] ?? false);
  let dlls: DllRecord[] = $derived($gameDlls[game.id] ?? []);
  let imgErrored = $state(false);
  let brandMark = $derived(launcherIcon(game.launcher));

  function isOutdated(r: DllRecord): boolean {
    return dllRelation(r, $relationContext) === "outdated";
  }

  let outdatedDlls = $derived(dlls.filter(isOutdated));
  let primaryOutdated = $derived(outdatedDlls[0] ?? null);
  let primaryTarget = $derived(primaryOutdated ? targetVersion(primaryOutdated, $relationContext) : null);

  let outdatedChips = $derived.by(() => {
    const seen = new Set<string>();
    const chips: { label: string; vendor: string }[] = [];
    for (const r of outdatedDlls) {
      const f = recordFeature(r);
      if (seen.has(f)) continue;
      seen.add(f);
      chips.push({
        label: f === "advanced" ? $t("feature.advanced.short") : featureShort(f),
        vendor: featureVendor(f),
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

</script>

<div
  class="list-row halo {haloVariant}"
  class:is-active={haloActive}
  class:is-hidden={hidden}
  role="presentation"
  data-launcher={game.launcher}
  oncontextmenu={onContextMenu ? (e) => { e.preventDefault(); onContextMenu(game, e); } : undefined}
>
  <div class="cover">
    {#if game.image_url && !imgErrored}
      <img src={game.image_url} alt={game.name} loading="lazy" onerror={() => (imgErrored = true)} />
    {:else}
      <span class="cover-fallback">{game.name.slice(0, 1).toUpperCase()}</span>
    {/if}
  </div>

  <div class="meta">
    <h3 class="row-name truncate" title={game.name}>
      <button
        class="row-name-btn truncate"
        aria-label={$t("component.card.rowAria", { name: game.name, status: $t("status." + status) })}
        onclick={() => onClick(game)}
      >{game.name}</button>
    </h3>
    <span class="launcher-chip chip">
      <svg viewBox={brandMark.viewBox} width="10" height="10" fill="currentColor" aria-hidden="true">
        <path d={brandMark.path} />
      </svg>
      {launcherLabel(game.launcher)}
    </span>
  </div>

  <div class="status">
    {#if loading}
      <span class="chip chip-neutral">{$t("status.scanning")}</span>
    {:else if status === "outdated"}
      <span class="chip chip-update is-strong"
        ><span class="state-dot" data-state="outdated" aria-hidden="true"></span
        >{$t("component.card.updatesShort", { count: outdatedDlls.length })}</span
      >
    {:else if status === "up_to_date"}
      <span class="chip chip-success"
        ><span class="state-dot" data-state="current" aria-hidden="true"></span
        >{$t("status.up_to_date")}</span
      >
    {:else if status === "scan_failed"}
      <span class="chip chip-danger">{$t("status.scan_failed")}</span>
    {:else if status === "no_dlls"}
      <span class="chip chip-neutral">{$t("status.no_dlls")}</span>
    {:else}
      <span class="chip chip-neutral">{$t("status.unknown")}</span>
    {/if}
  </div>

  <div class="chips">
    {#each visibleChips as c, i (i)}
      <span class="chip chip-vendor is-solid feature-chip" data-vendor={c.vendor}>{c.label}</span>
    {/each}
    {#if overflowChips > 0}
      <span class="feature-chip overflow">+{overflowChips}</span>
    {/if}
  </div>

  <div class="version-diff mono" title={$t("component.card.versionDiffTitle")}>
    {#if primaryOutdated && primaryTarget}
      <span class="ver-old">v{primaryOutdated.current_version ?? "?"}</span>
      <span class="arrow" aria-hidden="true">→</span>
      <span class="ver-new">v{primaryTarget}</span>
    {:else}
      <span class="ver-empty">—</span>
    {/if}
  </div>

  <div class="actions" onclick={(e) => e.stopPropagation()} role="presentation">
    {#if onToggleFavorite}
      <button
        class="btn btn-ghost btn-sm fav-row-btn"
        class:is-fav={favorite}
        aria-pressed={favorite}
        aria-label={favorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
        title={favorite ? $t("component.card.unfavorite") : $t("component.card.favorite")}
        onclick={() => onToggleFavorite?.(game)}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill={favorite ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
      </button>
    {/if}
    {#if status === "outdated" && !hidden}
      <button class="btn btn-primary btn-sm" onclick={() => onApply(game)} title={$t("component.card.applyLatest")}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        {$t("common.apply")}
      </button>
    {/if}
    <button class="btn btn-ghost btn-sm" onclick={() => onOpenFolder(game)} title={$t("component.card.openInstallFolder")} aria-label={$t("component.card.openInstallFolder")}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
    </button>
    <button class="btn btn-ghost btn-sm" onclick={() => onBlacklist(game)} title={hidden ? $t("component.card.restoreToLibrary") : $t("component.card.hideFromList")} aria-label={hidden ? $t("common.restore") : $t("component.card.hide")}>
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
    position: relative;
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
  .list-row:focus-within { border-color: var(--accent); box-shadow: var(--shadow-ring); }
  .list-row.is-hidden { opacity: 0.7; }
  .row-name-btn {
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
  .row-name-btn::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 1;
  }
  .row-name-btn:focus-visible { outline: none; }

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
    color: var(--launcher-chip-fg);
    text-transform: none;
    letter-spacing: 0;
    width: fit-content;
    border-color: transparent;
    background: var(--launcher-accent, #94a3b8);
  }

  .status { min-width: 100px; }

  .chips { display: flex; flex-wrap: nowrap; gap: 4px; overflow: hidden; min-width: 0; }
  .feature-chip { white-space: nowrap; }
  .feature-chip.overflow {
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    font-size: 10.5px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border);
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
    position: relative;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .fav-row-btn.is-fav { color: var(--gh-star); }
  .fav-row-btn.is-fav:hover { color: var(--gh-star); }

  @media (max-width: 1000px) {
    .list-row { grid-template-columns: 56px 1fr auto auto; gap: 10px; }
    .chips, .version-diff { display: none; }
  }
</style>
