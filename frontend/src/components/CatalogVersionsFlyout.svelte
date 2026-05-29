<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { listReleases, type Release } from "../lib/api";
  import {
    featureIconId,
    featureTitle,
    featureBlurb,
    familyLabel,
    familyShort,
    featureFromFamily,
    type FeatureSlot,
  } from "../lib/labels";
  import type { CatalogFamily } from "../lib/stores";
  import { showToast } from "../lib/stores";
  import { mergeFamilyReleases } from "../lib/catalogReleases";
  import FeatureIcon from "./FeatureIcon.svelte";
  import Checkbox from "./Checkbox.svelte";

  let {
    vendor,
    catalogKey,
    featureSlot,
    accent,
    families,
    advancedFamilies,
    vendorLabel,
    onClose,
  }: {
    vendor: string;
    catalogKey: string;
    featureSlot: FeatureSlot;
    accent: string;
    families?: string[];
    advancedFamilies?: CatalogFamily[];
    vendorLabel?: string;
    onClose: () => void;
  } = $props();

  type ViewMode = "modules" | "versions";

  let mode = $state<ViewMode>("versions");
  let activeFamilyLabel = $state<string>("");
  let activeFamilyIcon = $state<string>("advanced");

  let releases = $state<Release[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state("");
  let stableOnly = $state(true);

  let modulesQuery = $state("");

  let headerTitle = $derived.by(() => {
    if (mode === "modules") return `${vendorLabel ?? vendor} · Other technologies`;
    if (featureSlot === "advanced") return activeFamilyLabel;
    return featureTitle(featureSlot);
  });
  let headerSubtitle = $derived.by(() => {
    if (mode === "modules") return "Drill into any module to see every version, copy CDN URLs or download direct";
    if (featureSlot === "advanced") return `${vendorLabel ?? vendor} advanced technology`;
    return featureBlurb(featureSlot);
  });
  let headerIcon = $derived.by(() => (mode === "modules" ? "advanced" : activeFamilyIcon));

  onMount(() => {
    if (catalogKey === "advanced") {
      mode = "modules";
      activeFamilyLabel = "";
      activeFamilyIcon = "advanced";
    } else {
      mode = "versions";
      activeFamilyLabel = featureTitle(featureSlot);
      activeFamilyIcon = featureIconId(featureSlot);
      void loadFeatureVersions(families && families.length > 0 ? families : [catalogKey]);
    }
  });

  function toError(err: unknown): string {
    return err && typeof err === "object" && "message" in err
      ? String((err as { message: unknown }).message)
      : String(err);
  }

  async function loadFamilyVersions(family: string): Promise<void> {
    loading = true;
    error = null;
    releases = [];
    try {
      const list = await listReleases(vendor, family);
      list.sort((a, b) => Number(b.version_packed ?? 0) - Number(a.version_packed ?? 0));
      releases = list;
    } catch (err: unknown) {
      error = toError(err);
    } finally {
      loading = false;
    }
  }

  /// Load + merge every DLL family that maps to this feature (e.g. FSR Upscaling =
  /// DX12 + Vulkan), de-duped by version+hash and sorted newest-first.
  async function loadFeatureVersions(fams: string[]): Promise<void> {
    loading = true;
    error = null;
    releases = [];
    try {
      const lists = await Promise.all(fams.map((f) => listReleases(vendor, f)));
      releases = mergeFamilyReleases(lists);
    } catch (err: unknown) {
      error = toError(err);
    } finally {
      loading = false;
    }
  }

  function drillIntoFamily(f: CatalogFamily): void {
    activeFamilyLabel = familyLabel(f.family);
    activeFamilyIcon = featureIconId(featureFromFamily(f.family));
    mode = "versions";
    void loadFamilyVersions(f.family);
  }

  function backToModules(): void {
    if (catalogKey !== "advanced") return;
    mode = "modules";
    releases = [];
    error = null;
  }

  let filtered = $derived(
    releases.filter((r) => {
      if (stableOnly && r.channel !== "stable") return false;
      if (!query) return true;
      const q = query.toLowerCase();
      return (
        r.version.toLowerCase().includes(q) ||
        r.filename.toLowerCase().includes(q) ||
        (r.release_notes ?? "").toLowerCase().includes(q)
      );
    }),
  );
  let hiddenByStable = $derived(
    stableOnly ? releases.filter((r) => r.channel !== "stable").length : 0,
  );

  let filteredModules = $derived.by<CatalogFamily[]>(() => {
    const list = advancedFamilies ?? [];
    if (!modulesQuery.trim()) return list;
    const q = modulesQuery.toLowerCase();
    return list.filter((f) =>
      familyLabel(f.family).toLowerCase().includes(q) ||
      f.family.toLowerCase().includes(q),
    );
  });

  function formatDate(iso: string): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "—";
    return d.toISOString().slice(0, 10);
  }
  function formatSize(bytes: number): string {
    if (!bytes || bytes <= 0) return "—";
    const mb = bytes / (1024 * 1024);
    if (mb >= 1) return `${mb.toFixed(1)} MB`;
    return `${(bytes / 1024).toFixed(0)} KB`;
  }

  async function downloadRelease(r: Release): Promise<void> {
    if (!r.cdn_url) {
      showToast("warning", "No download URL for this version");
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(r.cdn_url);
      showToast("success", `Opening ${r.filename} (v${r.version})…`);
    } catch (err: unknown) {
      showToast("danger", `Open failed: ${String(err)}`);
    }
  }

  async function copyUrl(r: Release): Promise<void> {
    if (!r.cdn_url) {
      showToast("warning", "No URL on this release");
      return;
    }
    try {
      const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
      await writeText(r.cdn_url);
      showToast("success", `Copied URL for v${r.version}`);
    } catch (err: unknown) {
      showToast("danger", `Copy failed: ${String(err)}`);
    }
  }

  function handleKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.stopPropagation();
      if (mode === "versions" && catalogKey === "advanced") {
        backToModules();
      } else {
        onClose();
      }
    }
  }

  function moduleKey(e: KeyboardEvent, f: CatalogFamily): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      drillIntoFamily(f);
    }
  }
</script>

<div class="flyout-backdrop" role="presentation" onclick={onClose} onkeydown={handleKey} tabindex="-1"></div>
<div class="flyout glass-dialog" transition:fly={{ y: -8, duration: 160 }} style:--edge-color={accent} role="dialog" aria-label={headerTitle} tabindex="-1" onkeydown={handleKey}>
  <header class="flyout-head">
    {#if mode === "versions" && catalogKey === "advanced"}
      <button class="flyout-back" onclick={backToModules} aria-label="Back to modules">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
      </button>
    {/if}
    <div class="flyout-glyph" style:color={accent} aria-hidden="true">
      <FeatureIcon id={headerIcon} size={22} />
    </div>
    <div class="flyout-title">
      <span class="title-line">{headerTitle}</span>
      <span class="subtitle-line">{headerSubtitle}</span>
    </div>
    <button class="dialog-close" onclick={onClose} aria-label="Close">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
  </header>

  {#if mode === "modules"}
    <div class="flyout-toolbar">
      <input
        type="search"
        placeholder="Filter modules…"
        bind:value={modulesQuery}
        class="flyout-search"
      />
      <span class="toolbar-count">{filteredModules.length} of {(advancedFamilies ?? []).length} module{(advancedFamilies ?? []).length === 1 ? "" : "s"}</span>
    </div>
    <div class="flyout-body">
      {#if filteredModules.length === 0}
        <div class="flyout-state">
          <p>No modules match.</p>
        </div>
      {:else}
        <ul class="module-list">
          {#each filteredModules as f (f.family)}
            <li class="module-row">
              <button
                type="button"
                class="module-row-btn"
                onclick={() => drillIntoFamily(f)}
                onkeydown={(e) => moduleKey(e, f)}
                aria-label={`View versions of ${familyLabel(f.family)}`}
              >
                <span class="module-glyph" style:color={accent} aria-hidden="true">
                  <FeatureIcon id={featureIconId(featureFromFamily(f.family))} size={16} />
                </span>
                <div class="module-meta">
                  <span class="module-title">{familyLabel(f.family)}</span>
                  <span class="module-sub">{familyShort(f.family)} · {f.releaseCount} version{f.releaseCount === 1 ? "" : "s"}</span>
                </div>
                <span class="module-latest mono" style:color={accent}>v{f.latest}</span>
                <svg class="module-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else}
    <div class="flyout-toolbar">
      <input
        type="search"
        placeholder="Filter versions, files or notes…"
        bind:value={query}
        class="flyout-search"
      />
      <Checkbox
        bind:checked={stableOnly}
        label={`Stable only${hiddenByStable > 0 ? ` (-${hiddenByStable})` : ""}`}
      />
    </div>
    <div class="flyout-body">
      {#if loading}
        <div class="flyout-state">
          <span class="spinner"></span>
          <span>Loading versions…</span>
        </div>
      {:else if error}
        <div class="flyout-state danger">Failed to load: {error}</div>
      {:else if releases.length === 0}
        <div class="flyout-state">
          <p><strong>No versions tracked yet.</strong></p>
          <p class="small">This technology doesn't have any tracked upstream releases in DLSSync's manifest yet.</p>
        </div>
      {:else if filtered.length === 0}
        <div class="flyout-state">
          <p>No matches.</p>
          <p class="small">Clear the filter or include beta versions.</p>
        </div>
      {:else}
        <ul class="version-list">
          {#each filtered as r, i (r.version + r.sha256)}
            <li class="version-row" class:is-first={i === 0}>
              <div class="row-left">
                {#if i === 0}<span class="latest-tag">Latest</span>{/if}
                <span class="row-version mono">v{r.version}</span>
              </div>
              <div class="row-mid">
                <span class="row-file mono">{r.filename}</span>
                <span class="row-meta">
                  <span class="row-date">{formatDate(r.released_at)}</span>
                  <span class="row-sep">·</span>
                  <span class="row-size">{formatSize(r.size_bytes)}</span>
                  {#if r.channel === "experimental"}
                    <span class="row-sep">·</span>
                    <span class="chip chip-warning small-chip">Beta</span>
                  {/if}
                  {#if r.signed}
                    <span class="row-shield" title={r.signature_subject ?? "Signed by vendor"}>
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>
                    </span>
                  {/if}
                </span>
                {#if r.release_notes}
                  <span class="row-notes truncate" title={r.release_notes}>{r.release_notes}</span>
                {/if}
              </div>
              <div class="row-actions">
                <button class="btn btn-sm btn-accent" onclick={() => downloadRelease(r)} title="Open download URL in browser">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                  Download
                </button>
                <button class="btn btn-sm btn-ghost" onclick={() => copyUrl(r)} title="Copy CDN URL to clipboard">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <footer class="flyout-foot">
      <span class="foot-count">{filtered.length} of {releases.length} version{releases.length === 1 ? "" : "s"}</span>
    </footer>
  {/if}
</div>

<style>
  .flyout-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(3px);
    z-index: 220;
  }
  .flyout {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(720px, 94vw);
    max-height: 84vh;
    display: flex;
    flex-direction: column;
    z-index: 221;
  }
  .flyout-head {
    padding: 18px 52px 14px 20px;
    border-bottom: 1px solid var(--border);
    display: grid;
    grid-template-columns: auto 44px 1fr;
    gap: 14px;
    align-items: center;
  }
  .flyout-head:has(.flyout-back) { grid-template-columns: 30px 44px 1fr; }
  .flyout-head:not(:has(.flyout-back)) { grid-template-columns: 44px 1fr; }
  .flyout-back {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
  }
  .flyout-back:hover { background: var(--bg-elevated); color: var(--text-primary); }
  .flyout-glyph {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .flyout-title { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .title-line { font-size: var(--fs-lg); font-weight: 700; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .subtitle-line { font-size: var(--fs-sm); color: var(--text-secondary); line-height: 1.4; }

  .flyout-toolbar {
    padding: 10px 20px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-input);
  }
  .flyout-search { flex: 1; min-width: 160px; font-size: var(--fs-sm); padding: 7px 12px; }
  .toolbar-count { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .flyout-body { flex: 1; overflow-y: auto; }
  .flyout-state {
    padding: 50px 20px;
    text-align: center;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .flyout-state.danger { color: var(--danger); }
  .flyout-state .small { font-size: var(--fs-xs); opacity: 0.85; max-width: 360px; }

  .module-list { list-style: none; padding: 8px 0; margin: 0; }
  .module-row { padding: 0; }
  .module-row-btn {
    width: 100%;
    display: grid;
    grid-template-columns: 28px 1fr auto 14px;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    background: none;
    border: none;
    border-left: 2px solid transparent;
    text-align: left;
    color: inherit;
    cursor: pointer;
    transition: background 0.12s var(--ease), border-color 0.12s var(--ease);
  }
  .module-row-btn:hover { background: var(--bg-elevated); border-left-color: var(--accent); }
  .module-glyph { display: inline-flex; }
  .module-meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .module-title { font-size: var(--fs-md); color: var(--text-primary); font-weight: 600; letter-spacing: var(--letter-tight); }
  .module-sub { font-size: var(--fs-xs); color: var(--text-muted); text-transform: uppercase; letter-spacing: var(--letter-wider); font-weight: 600; }
  .module-latest { font-size: var(--fs-sm); font-variant-numeric: tabular-nums; font-weight: 700; }
  .module-arrow { color: var(--text-muted); opacity: 0.5; transition: transform 0.12s var(--ease), opacity 0.12s var(--ease); }
  .module-row-btn:hover .module-arrow { opacity: 1; transform: translateX(2px); color: var(--accent); }

  .version-list { list-style: none; padding: 6px 0 10px; margin: 0; }
  .version-row {
    display: grid;
    grid-template-columns: 130px 1fr auto;
    align-items: center;
    gap: 14px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
  }
  .version-row.is-first { background: var(--accent-soft); }
  .version-row:last-child { border-bottom: none; }

  .row-left { display: flex; flex-direction: column; gap: 2px; }
  .latest-tag {
    font-size: var(--fs-2xs);
    font-weight: 700;
    color: var(--accent);
    letter-spacing: var(--letter-wider);
    text-transform: uppercase;
  }
  .row-version { font-size: var(--fs-md); font-weight: 700; color: var(--text-primary); font-variant-numeric: tabular-nums; }

  .row-mid { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .row-file { font-size: var(--fs-xs); color: var(--text-muted); }
  .row-meta { display: inline-flex; align-items: center; gap: 6px; font-size: var(--fs-2xs); color: var(--text-muted); }
  .row-sep { opacity: 0.5; }
  .row-date, .row-size { font-variant-numeric: tabular-nums; }
  .row-shield { color: var(--success); display: inline-flex; }
  .row-notes { font-size: var(--fs-xs); color: var(--text-muted); opacity: 0.85; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .small-chip { padding: 1px 6px; font-size: var(--fs-2xs); letter-spacing: 0.04em; }
  .row-actions { display: inline-flex; gap: 6px; flex-shrink: 0; }

  .flyout-foot {
    padding: 10px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg-input);
  }
  .foot-count { font-size: var(--fs-xs); color: var(--text-muted); font-variant-numeric: tabular-nums; }

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
