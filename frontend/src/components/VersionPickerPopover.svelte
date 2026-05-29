<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { listReleases, type Release, type DllFamily } from "../lib/api";
  import {
    familyVendor,
    familyCatalogKey,
    familyShort,
    featureTitle,
    featureBlurb,
    featureIconId,
    recordFeature,
  } from "../lib/labels";
  import { settings } from "../lib/stores";
  import FeatureIcon from "./FeatureIcon.svelte";

  let {
    family,
    filename,
    currentVersion,
    latestVersion,
    pickedVersion,
    onPick,
    onClose,
  }: {
    family: DllFamily;
    filename: string;
    currentVersion: string | null;
    latestVersion: string | null;
    pickedVersion: string | null;
    onPick: (version: string | null) => void;
    onClose: () => void;
  } = $props();

  let releases = $state<Release[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let stableOnly = $state($settings?.advanced.prefer_stable_channel ?? true);
  let showOlder = $state(false);

  let feature = $derived(recordFeature({ family, path: filename, current_version: null, file_description: null, sha256: null }));
  let title = $derived(feature === "advanced" ? `${familyShort(family)}` : featureTitle(feature));
  let subtitle = $derived(feature === "advanced" ? "Advanced technology — manual control" : featureBlurb(feature));
  let icon = $derived(featureIconId(feature));

  onMount(async () => {
    try {
      const vendor = familyVendor(family);
      const key = familyCatalogKey(family);
      const list = await listReleases(vendor, key);
      list.sort((a, b) => packed(b) - packed(a));
      releases = list;
    } catch (err: unknown) {
      const msg =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err);
      error = msg;
    } finally {
      loading = false;
    }
  });

  function packed(r: Release): number {
    return Number(r.version_packed ?? 0);
  }
  function packedFromString(v: string): number {
    const parts = v.split(".").map((n) => parseInt(n, 10) || 0);
    const major = BigInt(parts[0] ?? 0);
    const minor = BigInt(parts[1] ?? 0);
    const build = BigInt(parts[2] ?? 0);
    const patch = BigInt(parts[3] ?? 0);
    return Number((major << 48n) | (minor << 32n) | (build << 16n) | patch);
  }

  let currentPacked = $derived(currentVersion ? packedFromString(currentVersion) : 0);
  let currentInCatalog = $derived(
    !!currentVersion && releases.some((r) => r.version === currentVersion),
  );

  let filtered = $derived(
    releases.filter((r) => {
      if (stableOnly && r.channel !== "stable") return false;
      if (!query) return true;
      const q = query.toLowerCase();
      return (
        r.version.toLowerCase().includes(q) ||
        (r.release_notes ?? "").toLowerCase().includes(q)
      );
    }),
  );

  type RankedRow = { release: Release; relation: "current" | "newer" | "older" };
  let ranked = $derived.by<RankedRow[]>(() =>
    filtered.map((r) => {
      const cmp = currentVersion ? packedFromString(r.version) - currentPacked : 1;
      const relation = cmp === 0 ? "current" : cmp > 0 ? "newer" : "older";
      return { release: r, relation };
    }),
  );

  let newerRows = $derived(ranked.filter((r) => r.relation === "newer"));
  let currentRow = $derived(ranked.find((r) => r.relation === "current"));
  let olderRows = $derived(ranked.filter((r) => r.relation === "older"));

  let hiddenByStable = $derived(
    stableOnly ? releases.filter((r) => r.channel !== "stable").length : 0,
  );

  let recommendedVersion = $derived(latestVersion ?? newerRows[0]?.release.version ?? null);
  let recommendedIsNewer = $derived(
    !!recommendedVersion &&
      !!currentVersion &&
      packedFromString(recommendedVersion) > currentPacked,
  );

  function formatDate(iso: string): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "—";
    return d.toISOString().slice(0, 10);
  }

  function vendorPortal(): { label: string; url: string } {
    const v = familyVendor(family);
    switch (v) {
      case "nvidia": return { label: "NVIDIA Developer DLSS", url: "https://developer.nvidia.com/rtx/dlss" };
      case "intel":  return { label: "Intel XeSS Releases", url: "https://github.com/intel/xess/releases" };
      case "amd":    return { label: "AMD GPUOpen FSR SDK", url: "https://gpuopen.com/amd-fidelityfx-sdk/" };
      case "microsoft": return { label: "DirectStorage NuGet", url: "https://www.nuget.org/packages/Microsoft.Direct3D.DirectStorage" };
      default:       return { label: "Upstream source", url: "" };
    }
  }

  async function openExternal(url: string): Promise<void> {
    if (!url) return;
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch {
      window.open(url, "_blank");
    }
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
  }

  function pick(version: string | null): void {
    onPick(version);
    onClose();
  }
</script>

<div class="picker-backdrop" role="presentation" onclick={onClose} onkeydown={handleKey} tabindex="-1"></div>
<div class="picker glass-dialog" transition:fly={{ y: -6, duration: 140 }} onkeydown={handleKey} role="dialog" aria-label="Pick version" tabindex="-1">
  <header class="picker-head">
    <div class="picker-glyph" aria-hidden="true">
      <FeatureIcon id={icon} size={20} />
    </div>
    <div class="picker-title">
      <span class="title-line">{title}</span>
      <span class="subtitle-line">{subtitle}</span>
      <span class="file-line mono">{filename}</span>
    </div>
    <button class="dialog-close" onclick={onClose} aria-label="Close">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
  </header>

  <section class="rank rank-1" aria-label="Currently installed">
    <span class="rank-eyebrow">Currently installed</span>
    <div class="rank-1-row">
      <span class="installed-version mono">v{currentVersion ?? "—"}</span>
      {#if !currentVersion}
        <span class="chip chip-neutral">Unknown</span>
      {:else if !currentInCatalog && !loading}
        <span class="chip chip-neutral">Not in catalog</span>
      {:else if recommendedIsNewer}
        <span class="chip chip-update is-strong">Update available</span>
      {:else}
        <span class="chip chip-success">Up to date</span>
      {/if}
    </div>
  </section>

  {#if recommendedVersion}
    <section class="rank rank-2" aria-label="Recommended">
      <span class="rank-eyebrow accent">{recommendedIsNewer ? "Recommended update" : "Vendor recommended"}</span>
      <button class="rec-tile" class:is-current={!recommendedIsNewer} onclick={() => pick(null)}>
        <div class="rec-meta">
          <span class="rec-label">Latest stable</span>
          <span class="rec-version mono">v{recommendedVersion}</span>
          <span class="rec-sub">Auto · upstream vendor recommended</span>
        </div>
        <div class="rec-cta">
          {#if recommendedIsNewer}
            <span class="rec-cta-text">Use latest</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
          {:else}
            <span class="rec-cta-text">Keep current</span>
          {/if}
        </div>
      </button>
    </section>
  {/if}

  <div class="picker-toolbar">
    <input
      type="search"
      placeholder="Filter versions or notes…"
      bind:value={query}
      class="picker-search"
    />
    <label class="picker-toggle" title="Hide beta and experimental builds">
      <input type="checkbox" bind:checked={stableOnly} />
      <span>Stable only{hiddenByStable > 0 ? ` (-${hiddenByStable})` : ""}</span>
    </label>
    <label class="picker-toggle" title="Include versions older than the currently installed one">
      <input type="checkbox" bind:checked={showOlder} />
      <span>Show older{olderRows.length > 0 ? ` (${olderRows.length})` : ""}</span>
    </label>
  </div>

  <section class="rank rank-3" aria-label="All versions">
    {#if loading}
      <div class="picker-state">
        <span class="spinner"></span>
        <span>Loading release history…</span>
      </div>
    {:else if error}
      <div class="picker-state danger">Failed to load: {error}</div>
    {:else if releases.length === 0}
      <div class="picker-state">
        <p><strong>No upstream catalog yet for this file.</strong></p>
        <p class="small">{filename} doesn't have a tracked manifest source in DLSSync yet. Check the vendor portal for fresh releases.</p>
        {#if vendorPortal().url}
          <button class="btn btn-sm btn-accent" onclick={() => openExternal(vendorPortal().url)}>
            {vendorPortal().label}
          </button>
        {/if}
      </div>
    {:else}
      {#if newerRows.length > 0}
        <div class="group-head">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>
          Newer versions
          <span class="group-count">{newerRows.length}</span>
        </div>
        <ul class="picker-list">
          {#each newerRows as r (r.release.version + r.release.sha256)}
            {@const isPicked = pickedVersion === r.release.version}
            <li class="picker-row newer" class:active={isPicked}>
              <button class="picker-row-btn" onclick={() => pick(r.release.version)}>
                <span class="rank-marker newer-marker" aria-hidden="true">↑</span>
                <div class="row-main">
                  <span class="row-version mono">v{r.release.version}</span>
                  <span class="row-meta-line">
                    <span class="row-date">{formatDate(r.release.released_at)}</span>
                    {#if r.release.release_notes}
                      <span class="row-notes truncate" title={r.release.release_notes}>· {r.release.release_notes}</span>
                    {/if}
                  </span>
                </div>
                <div class="row-tags">
                  {#if r.release.channel === "experimental"}<span class="chip chip-warning small-chip">Beta</span>{/if}
                  {#if r.release.signed}
                    <span class="row-shield" title={r.release.signature_subject ?? "Signed by vendor"}>
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>
                    </span>
                  {/if}
                  <span class="chip chip-update small-chip">Newer</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if currentRow}
        {@const r = currentRow.release}
        <div class="group-head current">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2" fill="currentColor"/></svg>
          Currently installed
        </div>
        <ul class="picker-list">
          <li class="picker-row current" class:active={pickedVersion === r.version}>
            <button class="picker-row-btn" onclick={() => pick(r.version)}>
              <span class="rank-marker current-marker" aria-hidden="true">●</span>
              <div class="row-main">
                <span class="row-version mono">v{r.version}</span>
                <span class="row-meta-line">
                  <span class="row-date">{formatDate(r.released_at)}</span>
                  {#if r.release_notes}
                    <span class="row-notes truncate" title={r.release_notes}>· {r.release_notes}</span>
                  {/if}
                </span>
              </div>
              <div class="row-tags">
                {#if r.signed}
                  <span class="row-shield" title={r.signature_subject ?? "Signed by vendor"}>
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>
                  </span>
                {/if}
                <span class="chip chip-info small-chip">Installed</span>
              </div>
            </button>
          </li>
        </ul>
      {/if}

      {#if olderRows.length > 0 && showOlder}
        <div class="group-head older">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><polyline points="19 12 12 19 5 12"/></svg>
          Older versions
          <span class="group-count">{olderRows.length}</span>
        </div>
        <ul class="picker-list">
          {#each olderRows as r (r.release.version + r.release.sha256)}
            {@const isPicked = pickedVersion === r.release.version}
            <li class="picker-row older" class:active={isPicked}>
              <button class="picker-row-btn" onclick={() => pick(r.release.version)}>
                <span class="rank-marker older-marker" aria-hidden="true">↓</span>
                <div class="row-main">
                  <span class="row-version mono">v{r.release.version}</span>
                  <span class="row-meta-line">
                    <span class="row-date">{formatDate(r.release.released_at)}</span>
                    {#if r.release.release_notes}
                      <span class="row-notes truncate" title={r.release.release_notes}>· {r.release.release_notes}</span>
                    {/if}
                  </span>
                </div>
                <div class="row-tags">
                  {#if r.release.channel === "experimental"}<span class="chip chip-warning small-chip">Beta</span>{/if}
                  <span class="chip chip-neutral small-chip">Older</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if newerRows.length === 0 && !currentRow && (olderRows.length === 0 || !showOlder)}
        <div class="picker-state">
          <p>No matches in this view.</p>
          <p class="small">Clear the filter, enable "Stable only" off, or toggle "Show older".</p>
        </div>
      {/if}
    {/if}
  </section>

  <footer class="picker-foot">
    <span class="foot-count">{filtered.length} of {releases.length} version{releases.length === 1 ? "" : "s"} matched</span>
  </footer>
</div>

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.38);
    z-index: 220;
    backdrop-filter: blur(2px);
  }
  .picker {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(600px, 92vw);
    max-height: 84vh;
    display: flex;
    flex-direction: column;
    z-index: 221;
  }
  .picker-head {
    padding: 16px 48px 16px 18px;
    border-bottom: 1px solid var(--border);
    display: grid;
    grid-template-columns: 40px 1fr;
    gap: 12px;
    align-items: flex-start;
  }
  .picker-glyph {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .picker-title { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .title-line { font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .subtitle-line { font-size: 11.5px; color: var(--text-secondary); line-height: 1.4; }
  .file-line { font-size: 10px; color: var(--text-muted); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .rank {
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
  }
  .rank-eyebrow {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    font-weight: 700;
    display: block;
    margin-bottom: 6px;
  }
  .rank-eyebrow.accent { color: var(--accent); }
  .rank-1 { background: var(--bg-input); }
  .rank-1-row { display: flex; align-items: center; gap: 10px; }
  .installed-version {
    font-size: var(--fs-xl-plus);
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--letter-tighter);
  }

  .rank-2 { background: var(--bg-card); }
  .rec-tile {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    background: linear-gradient(135deg, var(--accent-dim) 0%, transparent 70%);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    cursor: pointer;
    transition: transform 0.1s var(--ease), border-color 0.15s var(--ease), background 0.15s var(--ease);
    text-align: left;
  }
  .rec-tile:hover {
    transform: translateY(-1px);
    background: linear-gradient(135deg, var(--accent-glow) 0%, var(--accent-dim) 70%);
  }
  .rec-tile.is-current {
    background: transparent;
    border-color: var(--border-strong);
  }
  .rec-tile.is-current:hover { background: var(--bg-elevated); }
  .rec-meta { display: flex; flex-direction: column; gap: 2px; }
  .rec-label { font-size: 10px; font-weight: 700; color: var(--accent); letter-spacing: var(--letter-wider); text-transform: uppercase; }
  .rec-tile.is-current .rec-label { color: var(--text-muted); }
  .rec-version { font-size: 19px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: var(--letter-tighter); }
  .rec-sub { font-size: 11px; color: var(--text-secondary); }
  .rec-cta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: var(--radius-md);
    font-size: var(--fs-sm);
    font-weight: 700;
    letter-spacing: var(--letter-tight);
  }
  .rec-tile.is-current .rec-cta {
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }

  .picker-toolbar {
    padding: 10px 18px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
    background: var(--bg-input);
  }
  .picker-search { flex: 1; min-width: 160px; font-size: var(--fs-sm); padding: 6px 10px; }
  .picker-toggle { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-secondary); cursor: pointer; user-select: none; }
  .picker-toggle input { accent-color: var(--accent); }

  .rank-3 { flex: 1; overflow-y: auto; padding: 0; }
  .group-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 18px 4px;
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--update);
    font-weight: 700;
    position: sticky;
    top: 0;
    background: var(--bg-card);
    z-index: 1;
  }
  .group-head.current { color: var(--info); }
  .group-head.older { color: var(--text-muted); }
  .group-count {
    font-size: var(--fs-2xs);
    background: var(--bg-elevated);
    color: var(--text-muted);
    padding: 1px 7px;
    border-radius: var(--radius-full);
    font-weight: 600;
  }
  .picker-state {
    padding: 30px 18px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .picker-state.danger { color: var(--danger); }
  .picker-state .small { font-size: 10.5px; opacity: 0.85; max-width: 360px; }

  .picker-list { list-style: none; padding: 2px 0 6px; margin: 0; }
  .picker-row { padding: 0; }
  .picker-row-btn {
    width: 100%;
    display: grid;
    grid-template-columns: 22px 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 9px 18px;
    background: none;
    border: none;
    text-align: left;
    color: inherit;
    cursor: pointer;
    border-left: 2px solid transparent;
  }
  .picker-row-btn:hover { background: var(--bg-elevated); }
  .picker-row.newer .picker-row-btn { border-left-color: rgba(34, 211, 238, 0.45); }
  .picker-row.current .picker-row-btn { border-left-color: var(--info); background: rgba(124, 185, 232, 0.06); }
  .picker-row.older .picker-row-btn { border-left-color: rgba(71, 80, 99, 0.5); }
  .picker-row.active .picker-row-btn { background: var(--accent-soft); border-left-color: var(--accent); }

  .rank-marker {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    border-radius: var(--radius-full);
  }
  .newer-marker { color: var(--update); background: var(--update-dim); }
  .current-marker { color: var(--info); background: rgba(124, 185, 232, 0.18); }
  .older-marker { color: var(--text-muted); background: var(--neutral-dim); }

  .row-main { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .row-version { font-size: 13px; font-weight: 700; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .row-meta-line { display: flex; gap: 6px; min-width: 0; align-items: center; }
  .row-date { font-size: 10.5px; color: var(--text-muted); font-variant-numeric: tabular-nums; flex-shrink: 0; }
  .row-notes { font-size: 10px; color: var(--text-muted); opacity: 0.85; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .row-tags { display: inline-flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .row-shield { color: var(--success); display: inline-flex; }
  .small-chip { padding: 2px 7px; font-size: 9.5px; letter-spacing: 0.04em; }

  .picker-foot {
    padding: 9px 18px;
    border-top: 1px solid var(--border);
    background: var(--bg-input);
  }
  .foot-count { font-size: 10.5px; color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
