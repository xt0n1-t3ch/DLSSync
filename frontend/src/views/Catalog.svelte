<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import {
    catalogVendors,
    loadCatalog,
    manifestUpdatedAt,
    catalogStatus,
    driverReports,
    driverCheckInProgress,
    loadDriverUpdates,
    type CatalogFamily,
  } from "../lib/stores";
  import { driverStatusLabel, driverStatusTone } from "../lib/drivers";
  import {
    featureFromFamily,
    featureTitle,
    featureIconId,
    FEATURE_ORDER,
    GROUP_LABELS,
    vendorAccent,
    vendorPortal,
    type FeatureSlot,
  } from "../lib/labels";
  import FeatureIcon from "./../components/FeatureIcon.svelte";
  import CatalogVersionsFlyout from "./../components/CatalogVersionsFlyout.svelte";
  import BrandMark from "./../components/BrandMark.svelte";

  let refreshing = $state(false);
  let flyoutTarget = $state<{
    vendor: string;
    vendorLabel: string;
    catalogKey: string;
    featureSlot: FeatureSlot;
    accent: string;
    families: string[];
    advancedFamilies?: CatalogFamily[];
  } | null>(null);

  onMount(() => {
    void loadCatalog();
    void loadDriverUpdates();
  });

  function compareSemver(a: string, b: string): number {
    const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
    const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
    const len = Math.max(pa.length, pb.length);
    for (let i = 0; i < len; i++) {
      const x = pa[i] ?? 0;
      const y = pb[i] ?? 0;
      if (x < y) return -1;
      if (x > y) return 1;
    }
    return 0;
  }
  async function refresh(): Promise<void> {
    if (refreshing) return;
    refreshing = true;
    try { await loadCatalog(); } finally { refreshing = false; }
  }

  type Row = { id: string; iconId: string; title: string; latest: string; releaseCount: number; isAdvanced: boolean; featureSlot: FeatureSlot; catalogKey: string; families: string[]; advancedFamilies?: CatalogFamily[] };
  type VendorView = { vendor: string; label: string; accent: string; rows: Row[]; advancedCount: number; totalReleases: number };

  let view = $derived.by<VendorView[]>(() => {
    const out: VendorView[] = [];
    for (const v of $catalogVendors) {
      const rows: Row[] = [];
      const advFamiliesList: CatalogFamily[] = [];
      let advReleases = 0;
      let totalReleases = 0;
      const featureOrder = [...FEATURE_ORDER, "advanced"] as const;
      const byFeature = new Map<string, Row>();
      for (const f of v.families) {
        totalReleases += f.releaseCount;
        const slot = featureFromFamily(f.family);
        if (slot === "advanced") {
          advFamiliesList.push(f);
          advReleases += f.releaseCount;
        } else {
          const existing = byFeature.get(slot);
          if (existing) {
            existing.releaseCount += f.releaseCount;
            existing.families.push(f.family);
            if (compareSemver(f.latest, existing.latest) > 0) existing.latest = f.latest;
          } else {
            byFeature.set(slot, {
              id: slot,
              iconId: featureIconId(slot),
              title: featureTitle(slot),
              latest: f.latest,
              releaseCount: f.releaseCount,
              isAdvanced: false,
              featureSlot: slot,
              catalogKey: f.family,
              families: [f.family],
            });
          }
        }
      }
      for (const fid of featureOrder) {
        const row = byFeature.get(fid);
        if (row) rows.push(row);
      }
      if (advFamiliesList.length > 0) {
        rows.push({
          id: `${v.vendor}-advanced`,
          iconId: "advanced",
          title: GROUP_LABELS.advanced,
          latest: `${advReleases} total`,
          releaseCount: advFamiliesList.length,
          isAdvanced: true,
          featureSlot: "advanced",
          catalogKey: "advanced",
          families: [],
          advancedFamilies: advFamiliesList.slice().sort((a, b) => b.releaseCount - a.releaseCount),
        });
      }
      out.push({ vendor: v.vendor, label: v.label, accent: vendorAccent(v.vendor), rows, advancedCount: advFamiliesList.length, totalReleases });
    }
    return out;
  });

  function openFlyout(vendorKey: string, vendorLabel: string, row: Row): void {
    flyoutTarget = {
      vendor: vendorKey,
      vendorLabel,
      catalogKey: row.catalogKey,
      featureSlot: row.featureSlot,
      accent: vendorAccent(vendorKey),
      families: row.families,
      advancedFamilies: row.isAdvanced ? row.advancedFamilies : undefined,
    };
  }

  function rowKey(e: KeyboardEvent, vendorKey: string, vendorLabel: string, row: Row): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openFlyout(vendorKey, vendorLabel, row);
    }
  }

  let runtimeQuery = $state("");

  let filteredView = $derived.by<VendorView[]>(() => {
    if (!runtimeQuery.trim()) return view;
    const q = runtimeQuery.trim().toLowerCase();
    return view
      .map((v) => ({
        ...v,
        rows: v.rows.filter((r) =>
          r.title.toLowerCase().includes(q) ||
          r.catalogKey.toLowerCase().includes(q) ||
          (r.advancedFamilies ?? []).some((af) => af.family.toLowerCase().includes(q) || af.label.toLowerCase().includes(q))
        ),
      }))
      .filter((v) => v.rows.length > 0 || v.label.toLowerCase().includes(q));
  });

  async function openExternal(url: string): Promise<void> {
    if (!url) return;
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch {
      window.open(url, "_blank");
    }
  }

  let totals = $derived.by(() => {
    let features = 0, releases = 0, vendors = view.length;
    for (const v of view) {
      features += v.rows.filter(r => !r.isAdvanced).length;
      releases += v.totalReleases;
    }
    return { features, releases, vendors };
  });

  let maxReleaseCount = $derived.by(() => {
    let m = 0;
    for (const v of view) m = Math.max(m, v.totalReleases);
    return m;
  });
  function bentoWeight(totalReleases: number): "lg" | "md" | "sm" {
    if (maxReleaseCount === 0) return "sm";
    const ratio = totalReleases / maxReleaseCount;
    if (ratio >= 0.8) return "lg";
    if (ratio >= 0.3) return "md";
    return "sm";
  }

  let freshnessAgo = $derived.by(() => {
    if (!$manifestUpdatedAt) return null;
    const parsed = new Date(`${$manifestUpdatedAt.replace(" ", "T")}:00Z`).getTime();
    if (Number.isNaN(parsed)) return null;
    const diffMs = Date.now() - parsed;
    const minutes = Math.floor(diffMs / 60000);
    if (minutes < 1) return "just now";
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  });
</script>

<div class="catalog-page">
<header class="view-header">
  <div>
    <h1 class="view-title">Catalog</h1>
    <p class="view-subtitle">Every upscaling and frame-generation technology DLSSync tracks. Click a feature to inspect every version, copy a CDN URL, or jump straight to a download.</p>
  </div>
  <button class="btn btn-primary" onclick={refresh} disabled={refreshing}>
    {#if refreshing}
      <span class="spin"></span>
      Refreshing
    {:else}
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
      Refresh manifest
    {/if}
  </button>
</header>

<section class="info-bar edge-accent" in:fade={{ duration: 200 }}>
  <div class="info-item">
    <span class="info-item-label">Versions</span>
    <span class="info-item-value">{totals.releases.toLocaleString()}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Features</span>
    <span class="info-item-value">{totals.features}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Vendors</span>
    <span class="info-item-value">{totals.vendors}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Updated</span>
    <span class="info-item-value is-mono">{$manifestUpdatedAt || "—"}</span>
  </div>
</section>

<section class="driver-catalog" in:fade={{ duration: 200 }}>
  <div class="driver-cat-head">
    <h2 class="driver-cat-title">GPU Drivers</h2>
    <span class="driver-cat-sub">Latest published driver per detected GPU, resolved live from the vendor.</span>
  </div>
  {#if $driverReports.length === 0}
    <p class="driver-cat-empty">{$driverCheckInProgress ? "Checking the vendor for the latest drivers…" : "No GPUs detected."}</p>
  {:else}
    <ul class="driver-cat-list">
      {#each $driverReports as report (report.device.model)}
        {@const tone = driverStatusTone(report.status)}
        <li class="driver-cat-row">
          <span class="driver-cat-vendor" data-vendor={report.device.vendor}>
            {#if report.device.vendor === "other"}
              GPU
            {:else}
              <BrandMark key={report.device.vendor} tone="mono" size={11} />
            {/if}
          </span>
          <span class="driver-cat-model">{report.device.model}</span>
          <span class="driver-cat-ver mono">
            {report.installed.display}
            {#if report.latest}<span class="driver-cat-arrow">→</span><span class="driver-cat-next">{report.latest.version.display}</span>{/if}
          </span>
          <span class="driver-cat-badge" data-tone={tone}>{driverStatusLabel(report.status)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if view.length === 0}
  <div class="empty">
    <p class="section-sub">
      Catalog is empty. The manifest worker hasn't published a payload yet — click <strong>Refresh manifest</strong> above to pull from upstream sources, or wait for the bundled cache to land.
    </p>
  </div>
{:else}
  <div class="section-head">
    <h2 class="section-title">Upscaling Libraries &amp; Technologies</h2>
    <span class="section-sub">Every DLSS, FSR and XeSS DLL family DLSSync tracks. Click one to browse every version, copy a CDN URL, or download direct.</span>
  </div>
  <div class="catalog-toolbar">
    <div class="runtime-search">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="search-icon"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
      <input
        type="search"
        placeholder="Filter technologies, vendors, or DLL families…"
        bind:value={runtimeQuery}
      />
      {#if runtimeQuery}
        <button class="search-clear" onclick={() => (runtimeQuery = "")} aria-label="Clear search">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      {/if}
    </div>
    <span class="toolbar-summary">{filteredView.reduce((a, v) => a + v.rows.length, 0)} technolog{filteredView.reduce((a, v) => a + v.rows.length, 0) === 1 ? "y" : "ies"} shown</span>
  </div>

  <div class="catalog-grid">
    {#each filteredView as v, i (v.vendor)}
      {@const portal = vendorPortal(v.vendor)}
      <section class="vendor-card" data-weight={bentoWeight(v.totalReleases)} in:fly={{ y: 12, duration: 320, delay: 80 + i * 50 }}>
        <div class="vendor-stripe" style:background={v.accent}></div>
        <header class="vendor-head">
          <div class="vendor-dot" style:background={v.accent} style:box-shadow="0 0 14px {v.accent}80"></div>
          <h3 class="vendor-name"><BrandMark key={v.vendor} label={v.label} size={16} /></h3>
          <span class="chip chip-neutral vendor-pill">{v.totalReleases} versions</span>
          {#if portal}
            <button class="vendor-portal" onclick={() => openExternal(portal.url)} title={portal.label} aria-label={`Open ${portal.label}`}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
            </button>
          {/if}
        </header>
        <ul class="feature-list">
          {#each v.rows as f (f.id)}
            <li class="feature-row">
              <button
                type="button"
                class="feature-row-btn"
                class:is-advanced={f.isAdvanced}
                onclick={() => openFlyout(v.vendor, v.label, f)}
                onkeydown={(e) => rowKey(e, v.vendor, v.label, f)}
                aria-label={f.isAdvanced ? `Browse ${f.title} modules` : `View ${f.title} versions`}
              >
                <span class="feature-glyph" style:color={v.accent} aria-hidden="true"><FeatureIcon id={f.iconId} size={16} /></span>
                <div class="feature-meta-col">
                  <span class="feature-title">{f.title}</span>
                  {#if f.isAdvanced}
                    <span class="feature-sub">{f.releaseCount} module{f.releaseCount === 1 ? "" : "s"} · {f.latest}</span>
                  {:else}
                    <span class="feature-sub">{f.releaseCount} version{f.releaseCount === 1 ? "" : "s"} · latest v{f.latest}</span>
                  {/if}
                </div>
                {#if !f.isAdvanced}
                  <span class="feature-version mono" style:color={v.accent}>v{f.latest}</span>
                {:else}
                  <span class="feature-version mono" style:color={v.accent}>{f.releaseCount}×</span>
                {/if}
                <svg class="feature-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>

  <footer class="catalog-foot">
    <div class="foot-status">
      <span class="status-dot is-{$catalogStatus.kind}" aria-hidden="true"></span>
      <span class="foot-status-text">Catalog {$catalogStatus.label}</span>
      {#if freshnessAgo}
        <span class="foot-sep" aria-hidden="true"></span>
        <span class="foot-meta">updated {freshnessAgo}</span>
      {/if}
    </div>
    <div class="foot-actions">
      <span class="foot-meta">auto-refresh every 6 h</span>
      <button class="foot-refresh" onclick={refresh} disabled={refreshing} title="Pull the manifest from upstream now">
        {#if refreshing}
          <span class="spin"></span>
          Refreshing
        {:else}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
          Refresh now
        {/if}
      </button>
    </div>
  </footer>
{/if}
</div>

{#if flyoutTarget}
  <CatalogVersionsFlyout
    vendor={flyoutTarget.vendor}
    vendorLabel={flyoutTarget.vendorLabel}
    catalogKey={flyoutTarget.catalogKey}
    featureSlot={flyoutTarget.featureSlot}
    accent={flyoutTarget.accent}
    families={flyoutTarget.families}
    advancedFamilies={flyoutTarget.advancedFamilies}
    onClose={() => (flyoutTarget = null)}
  />
{/if}

<style>
  .catalog-page {
    display: flex;
    flex-direction: column;
  }
  .view-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; gap: 12px; flex-wrap: wrap; }
  .view-header > div:first-child { flex: 1 1 240px; min-width: 0; }
  .empty { padding: 40px 0; text-align: center; }

  .driver-catalog {
    margin-bottom: 18px;
    padding: 16px 18px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .driver-cat-head { display: flex; align-items: baseline; gap: 12px; margin-bottom: 12px; flex-wrap: wrap; }
  .driver-cat-title { font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .driver-cat-sub { font-size: var(--fs-xs); color: var(--text-muted); }
  .driver-cat-empty { font-size: var(--fs-sm); color: var(--text-muted); padding: 8px 0; }
  .driver-cat-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .driver-cat-row {
    display: grid;
    grid-template-columns: 64px 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
  }
  .driver-cat-vendor { font-size: 10px; font-weight: 700; letter-spacing: 0.04em; color: var(--text-secondary); }
  .driver-cat-vendor[data-vendor="nvidia"] { color: var(--vendor-nvidia); }
  .driver-cat-vendor[data-vendor="amd"] { color: var(--vendor-amd); }
  .driver-cat-vendor[data-vendor="intel"] { color: var(--vendor-intel); }
  .driver-cat-model { font-size: var(--fs-sm); font-weight: 600; color: var(--text-primary); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .driver-cat-ver { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .driver-cat-arrow { margin: 0 5px; color: var(--text-muted); }
  .driver-cat-next { color: var(--accent); font-weight: 600; }
  .driver-cat-badge { font-size: 10.5px; font-weight: 600; padding: 3px 9px; border-radius: var(--radius-full); background: var(--bg-card); color: var(--text-muted); white-space: nowrap; }
  .driver-cat-badge[data-tone="success"] { background: var(--success-dim); color: var(--success); }
  .driver-cat-badge[data-tone="accent"] { background: var(--update-dim); color: var(--update); }
  .driver-cat-badge[data-tone="warning"] { background: var(--warning-dim); color: var(--warning); }


  .catalog-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .runtime-search { position: relative; flex: 1; max-width: 480px; display: flex; align-items: center; }
  .runtime-search input {
    width: 100%;
    padding: 9px 34px 9px 34px;
    border-radius: var(--radius-full);
    font-size: var(--fs-sm);
    background: var(--bg-input);
    border: 1px solid var(--border);
  }
  .runtime-search input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-dim); }
  .runtime-search .search-icon { position: absolute; left: 12px; color: var(--text-muted); pointer-events: none; }
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

  .section-head {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 14px;
    padding-top: 4px;
  }
  .section-title {
    font-size: var(--fs-lg);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    color: var(--text-primary);
  }
  .section-sub {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    max-width: 70ch;
  }
  .catalog-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
    align-items: start;
  }
  .vendor-portal {
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: color 0.12s var(--ease), background 0.12s var(--ease);
  }
  .vendor-portal:hover { color: var(--accent); background: var(--bg-elevated); }

  .vendor-card {
    position: relative;
    padding: 18px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: border-color 0.18s var(--ease), transform 0.18s var(--ease);
  }
  .vendor-card:hover { border-color: var(--border-hover); transform: translateY(-1px); }
  .vendor-stripe {
    position: absolute; top: 0; left: 0; right: 0; height: 2px; opacity: 0.7;
  }
  .vendor-head { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  .vendor-dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }
  .vendor-name { font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tight); flex: 1; }
  .vendor-pill { padding: 2px 8px; font-size: 9.5px; }

  .feature-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
  .feature-row { padding: 0; }
  .feature-row-btn {
    width: 100%;
    display: grid;
    grid-template-columns: 24px 1fr auto auto;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    border: 1px solid transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s var(--ease), border-color 0.12s var(--ease), transform 0.1s var(--ease);
  }
  .feature-row-btn:hover { background: var(--bg-card-hover); border-color: var(--border-hover); transform: translateX(2px); }
  .feature-row-btn:focus-visible { outline: none; border-color: var(--accent); box-shadow: var(--shadow-ring); }
  .feature-row-btn.is-advanced { background: transparent; border: 1px dashed var(--border); }
  .feature-row-btn.is-advanced:hover { background: var(--bg-card-hover); border-color: var(--border-hover); border-style: solid; }
  .feature-glyph { display: inline-flex; }
  .feature-meta-col { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .feature-title { font-size: var(--fs-sm); color: var(--text-primary); font-weight: 600; letter-spacing: var(--letter-tight); }
  .feature-sub { font-size: var(--fs-xs); color: var(--text-muted); text-transform: uppercase; letter-spacing: var(--letter-wider); font-weight: 600; }
  .feature-version { font-size: var(--fs-sm); font-variant-numeric: tabular-nums; font-weight: 600; }
  .feature-arrow { color: var(--text-muted); flex-shrink: 0; opacity: 0.4; transition: opacity 0.12s var(--ease), transform 0.12s var(--ease); }
  .feature-row-btn:hover .feature-arrow { opacity: 1; transform: translateX(2px); }

  .catalog-foot {
    margin-top: 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px 16px;
    flex-wrap: wrap;
    padding: 11px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    font-size: var(--fs-xs);
    color: var(--text-muted);
  }
  .foot-status { display: inline-flex; align-items: center; gap: 9px; min-width: 0; }
  .foot-status-text {
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: var(--letter-tight);
    text-transform: capitalize;
  }
  .foot-meta { font-variant-numeric: tabular-nums; color: var(--text-muted); }
  .foot-sep { width: 3px; height: 3px; border-radius: 50%; background: var(--text-muted); opacity: 0.4; flex-shrink: 0; }
  .foot-actions { display: inline-flex; align-items: center; gap: 14px; }
  .foot-refresh {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 14px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .foot-refresh:hover:not(:disabled) { background: var(--accent-soft); color: var(--accent); }
  .foot-refresh:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .foot-refresh:disabled { opacity: 0.6; cursor: progress; }

  .status-dot { position: relative; display: inline-block; width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .status-dot.is-accent { background: var(--accent); }
  .status-dot.is-success { background: var(--success); }
  .status-dot.is-warning { background: var(--warning); }
  .status-dot.is-danger { background: var(--danger); }
  @media (prefers-reduced-motion: no-preference) {
    .status-dot::after {
      content: "";
      position: absolute;
      inset: 0;
      border-radius: 50%;
      background: inherit;
      animation: status-pulse 2.4s var(--ease-out) infinite;
    }
  }
  @keyframes status-pulse {
    0% { transform: scale(1); opacity: 0.6; }
    70% { transform: scale(2.6); opacity: 0; }
    100% { transform: scale(2.6); opacity: 0; }
  }

  .spin { width: 12px; height: 12px; border: 2px solid currentColor; border-top-color: transparent; border-radius: 50%; animation: spin 0.7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
