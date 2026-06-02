<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { fly } from "svelte/transition";
  import { t, locale, translate } from "../lib/i18n/index";
  import Checkbox from "./Checkbox.svelte";
  import {
    driverHistory,
    driverHistoryLoading,
    loadDriverHistory,
    showToast,
  } from "../lib/stores";
  import { installDriver, openUrl, type DriverReleaseDto, type GpuVendor } from "../lib/api";
  import BrandMark from "./BrandMark.svelte";
  import { BRANDS } from "../lib/brands";

  let {
    vendor,
    model,
    accent,
    onClose,
  }: {
    vendor: GpuVendor;
    model: string;
    accent: string;
    onClose: () => void;
  } = $props();

  let vendorName = $derived(
    vendor === "other" ? $t("view.catalog.gpuDrivers.genericVendor") : BRANDS[vendor].label,
  );

  let query = $state("");
  let whqlOnly = $state(false);
  let installingVersion = $state<string | null>(null);

  onMount(() => {
    if (vendor === "nvidia" || vendor === "amd" || vendor === "intel") {
      void loadDriverHistory(model, vendor);
    }
  });

  let releases = $derived($driverHistory[model] ?? []);
  let loading = $derived($driverHistoryLoading[model] ?? false);

  let betaCount = $derived(releases.filter((r) => r.is_beta).length);
  let hiddenByWhql = $derived(whqlOnly ? betaCount : 0);
  let whqlToggleLabel = $derived.by(() => {
    if (betaCount === 0) return $t("component.flyout.whqlOnlyAllWhql");
    return hiddenByWhql > 0
      ? $t("component.flyout.whqlOnlyHidden", { count: hiddenByWhql })
      : $t("component.flyout.whqlOnly");
  });

  let filtered = $derived.by<DriverReleaseDto[]>(() => {
    const q = query.trim().toLowerCase();
    return releases.filter((r) => {
      if (whqlOnly && r.is_beta) return false;
      if (!q) return true;
      const blob = `${r.version.display} ${r.version.raw} ${r.display_version ?? ""} ${
        r.release_notes_url ?? ""
      }`.toLowerCase();
      return blob.includes(q);
    });
  });

  function sizeLabel(bytes: number): string {
    if (bytes <= 0) return "";
    const mb = bytes / (1024 * 1024);
    return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
  }

  function dateLabel(iso: string | null): string {
    if (!iso) return "";
    try {
      return new Date(iso).toISOString().slice(0, 10);
    } catch {
      return "";
    }
  }

  async function openNotes(url: string | null): Promise<void> {
    if (!url) return;
    try {
      await openUrl(url);
    } catch (err) {
      showToast("warning", translate(get(locale), "view.drivers.openLinkFailed", { error: String(err) }));
    }
  }

  async function install(release: DriverReleaseDto): Promise<void> {
    if (installingVersion || !release.download_url) return;
    installingVersion = release.version.display;
    try {
      const outcome = await installDriver(vendor, release.download_url);
      if (outcome.stage === "completed") {
        showToast("success", outcome.message);
      } else if (outcome.stage === "cancelled") {
        showToast("warning", outcome.message);
      } else {
        showToast("danger", outcome.message);
      }
    } catch (err) {
      showToast("danger", translate(get(locale), "component.flyout.installFailed", { error: String(err) }));
    } finally {
      installingVersion = null;
    }
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape") onClose();
  }
</script>

<div
  class="flyout-backdrop"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKey}
  tabindex="-1"
></div>
<div
  class="flyout glass-dialog"
  transition:fly={{ y: -8, duration: 160 }}
  style:--edge-color={accent}
  role="dialog"
  aria-label={$t("component.flyout.driverHistoryAria", { vendor: vendorName, model })}
  tabindex="-1"
  onkeydown={handleKey}
>
  <header class="flyout-head">
    <span class="vendor-pill" data-vendor={vendor} style:color={accent}>
      {#if vendor === "other"}
        {vendorName}
      {:else}
        <BrandMark key={vendor} tone="mono" size={12} />
      {/if}
    </span>
    <div class="flyout-title">
      <span class="title-line">{model}</span>
      <span class="subtitle-line">{$t("component.flyout.driverHistorySubtitle")}</span>
    </div>
    <button class="dialog-close" onclick={onClose} aria-label={$t("common.close")}>
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        ><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg
      >
    </button>
  </header>

  <div class="flyout-toolbar">
    <input
      type="search"
      placeholder={$t("component.flyout.filterVersionsNotes")}
      bind:value={query}
      class="flyout-search"
    />
    <Checkbox
      bind:checked={whqlOnly}
      disabled={betaCount === 0}
      label={whqlToggleLabel}
    />
  </div>

  <div class="flyout-body">
    {#if loading}
      <div class="flyout-state">
        <span class="spinner"></span>
        <span>{$t("component.flyout.loadingDriverHistory")}</span>
      </div>
    {:else if releases.length === 0}
      <div class="flyout-state">
        <p>{$t("component.flyout.noHistoricalDrivers")}</p>
      </div>
    {:else if filtered.length === 0}
      <div class="flyout-state">{$t("component.flyout.noVersionsMatchFilter")}</div>
    {:else}
      <ul class="release-list">
        {#each filtered as release, i (release.version.raw + i)}
          {@const size = sizeLabel(release.size_bytes)}
          {@const installing = installingVersion === release.version.display}
          {@const latest = i === 0 && !query && !whqlOnly}
          <li class="release-row">
            <div class="release-main">
              {#if latest}<span class="latest-badge" style:color={accent}>{$t("component.flyout.latestUpper")}</span>{/if}
              <span class="release-ver mono">v{release.version.display}</span>
              {#if release.is_beta}<span class="chan-chip beta">{$t("view.drivers.channelBeta")}</span>{:else}<span
                  class="chan-chip whql">{$t("view.drivers.channelWhql")}</span
                >{/if}
              {#if release.display_version}
                <span class="release-display">{release.display_version}</span>
              {/if}
            </div>
            <div class="release-meta">
              <span class="release-date mono">{dateLabel(release.released_at)}</span>
              {#if size}<span class="release-size mono">{size}</span>{/if}
              <span class="release-raw mono">{release.version.raw}</span>
            </div>
            <div class="release-actions">
              {#if release.release_notes_url}
                <button
                  class="link-btn"
                  onclick={() => openNotes(release.release_notes_url)}
                  title={$t("component.flyout.openReleaseNotesPageTitle")}
                >
                  {$t("component.flyout.notes")} ↗
                </button>
              {/if}
              {#if release.download_url}
                <button
                  class="get-btn"
                  onclick={() => install(release)}
                  disabled={!!installingVersion}
                >
                  {#if installing}<span class="spinner small"></span>{$t("component.flyout.installing")}{:else}{$t("common.install")}{/if}
                </button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <footer class="flyout-foot">
    <span>{$t("component.flyout.versionCount", { shown: filtered.length, count: releases.length })}</span>
  </footer>
</div>

<style>
  .flyout-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 80;
  }
  .flyout {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(720px, 92vw);
    max-height: 84vh;
    z-index: 81;
    display: flex;
    flex-direction: column;
  }
  .flyout-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 50px 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  .vendor-pill {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 4px 9px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    flex-shrink: 0;
  }
  .flyout-title { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .title-line { font-size: 15px; font-weight: 700; color: var(--text-primary); }
  .subtitle-line { font-size: 12px; color: var(--text-muted); }

  .flyout-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
  }
  .flyout-search {
    flex: 1;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 12.5px;
  }
  .flyout-search:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-dim); }

  .flyout-body { overflow-y: auto; padding: 6px 0; }
  .flyout-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 36px 18px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .release-list { list-style: none; padding: 0; margin: 0; }
  .release-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 14px;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.04));
  }
  .release-row:last-child { border-bottom: none; }
  .release-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex-wrap: wrap;
  }
  .latest-badge {
    font-size: 9.5px;
    font-weight: 800;
    letter-spacing: 0.08em;
    padding: 2px 7px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
  }
  .release-ver { font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .release-display { font-size: 11.5px; color: var(--text-muted); }
  .chan-chip {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: var(--radius-full);
    letter-spacing: 0.04em;
  }
  .chan-chip.whql { background: var(--success-dim, rgba(70, 180, 110, 0.14)); color: var(--success, #46b46e); }
  .chan-chip.beta { background: var(--warning-dim, rgba(214, 160, 50, 0.14)); color: var(--warning, #d6a032); }
  .release-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .release-actions { display: flex; align-items: center; gap: 6px; }
  .get-btn {
    height: 28px;
    padding: 0 12px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 11.5px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .get-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .get-btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .link-btn {
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 11.5px;
    font-weight: 600;
  }
  .link-btn:hover { color: var(--text-primary); }

  .flyout-foot {
    padding: 10px 18px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  .spinner.small { width: 10px; height: 10px; border-width: 1.5px; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
