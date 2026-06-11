<script lang="ts" module>
  import type { FeatureSlot } from "../lib/labels";
  import type { DllRecord } from "../lib/api";

  export type DrawerFeatureBucket = {
    feature: FeatureSlot;
    records: DllRecord[];
    primary: DllRecord;
    title: string;
    blurb: string;
    iconId: string;
    accent: string;
    anyOutdated: boolean;
    anyAhead: boolean;
    allUpToDate: boolean;
    allDisabled: boolean;
    statusLabel: string;
    statusTone: "update" | "success" | "info" | "neutral";
  };

  export type DrawerAdvancedRow = {
    family: string;
    label: string;
    records: DllRecord[];
    primary: DllRecord;
    anyOutdated: boolean;
  };

  type Relation = "outdated" | "same" | "ahead" | "no-target";
</script>

<script lang="ts">
  import { t } from "../lib/i18n/index";
  import { familyShort, GROUP_ACCENT, filenameFromPath } from "../lib/labels";
  import VersionPickerPopover from "./VersionPickerPopover.svelte";
  import FeatureIcon from "./FeatureIcon.svelte";
  import DlssOverridePanel from "./DlssOverridePanel.svelte";

  let {
    hasRecords,
    recordCount,
    outdatedCount,
    selectedCount,
    featureBuckets,
    advancedRows,
    selected,
    disabledFamilies,
    pinnedVersions,
    expandedFeatures,
    advancedExpanded,
    pickerOpenFor,
    dlssExpanded,
    dlssExe,
    dlssExeResolving,
    dlssDriverPacked,
    rowKey,
    relation,
    targetFor,
    latestFor,
    featureSelectionState,
    onSelectAllOutdated,
    onClearSelection,
    onToggleFeatureSelection,
    onToggleFileSelection,
    onToggleFeatureDisabled,
    onSetPin,
    onSetPickerOpen,
    onToggleFeatureExpanded,
    onToggleAdvanced,
    onToggleDlss,
    onRowContextMenu,
    onRowMenuAnchor,
  }: {
    hasRecords: boolean;
    recordCount: number;
    outdatedCount: number;
    selectedCount: number;
    featureBuckets: DrawerFeatureBucket[];
    advancedRows: DrawerAdvancedRow[];
    selected: Record<string, boolean>;
    disabledFamilies: string[];
    pinnedVersions: Record<string, string>;
    expandedFeatures: Record<string, boolean>;
    advancedExpanded: boolean;
    pickerOpenFor: string | null;
    dlssExpanded: boolean;
    dlssExe: string | null;
    dlssExeResolving: boolean;
    dlssDriverPacked: number;
    rowKey: (r: DllRecord) => string;
    relation: (r: DllRecord) => Relation;
    targetFor: (r: DllRecord) => string | null;
    latestFor: (r: DllRecord) => string | null;
    featureSelectionState: (b: DrawerFeatureBucket) => "all" | "some" | "none";
    onSelectAllOutdated: () => void;
    onClearSelection: () => void;
    onToggleFeatureSelection: (b: DrawerFeatureBucket, checked: boolean) => void;
    onToggleFileSelection: (key: string, checked: boolean) => void;
    onToggleFeatureDisabled: (recs: DllRecord[]) => void;
    onSetPin: (key: string, version: string | null) => void;
    onSetPickerOpen: (key: string | null) => void;
    onToggleFeatureExpanded: (feature: string) => void;
    onToggleAdvanced: () => void;
    onToggleDlss: () => void;
    onRowContextMenu: (primaryKey: string, e: MouseEvent) => void;
    onRowMenuAnchor: (primaryKey: string, x: number, y: number) => void;
  } = $props();

  function openMenuFromButton(primaryKey: string, e: MouseEvent): void {
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    onRowMenuAnchor(primaryKey, rect.left, rect.bottom + 4);
  }
</script>

{#if hasRecords}
<div class="summary-row">
  <div class="summary-stat">
    <span class="stat-num">{recordCount}</span>
    <span class="stat-label">{$t("component.gameDrawer.stat.files")}</span>
  </div>
  <div class="summary-stat">
    <span class="stat-num" class:is-update={outdatedCount > 0}>{outdatedCount}</span>
    <span class="stat-label">{$t("component.gameDrawer.stat.updates")}</span>
  </div>
  <div class="summary-stat">
    <span class="stat-num" class:is-accent={selectedCount > 0}>{selectedCount}</span>
    <span class="stat-label">{$t("component.gameDrawer.stat.selected")}</span>
  </div>
</div>

{#if outdatedCount > 0}
  <div class="quick-actions">
    <button class="btn btn-sm btn-accent" onclick={onSelectAllOutdated}>
      {$t("component.gameDrawer.selectAllUpdates", { count: outdatedCount })}
    </button>
    <button class="btn btn-sm btn-ghost" onclick={onClearSelection} disabled={selectedCount === 0}>
      {$t("component.gameDrawer.clearSelection")}
    </button>
  </div>
{/if}

{#if featureBuckets.length > 0}
  <ul class="feature-list stagger">
    {#each featureBuckets as b (b.feature)}
      {@const selState = featureSelectionState(b)}
      {@const expanded = !!expandedFeatures[b.feature]}
      {@const primaryKey = rowKey(b.primary)}
      {@const primaryRel = relation(b.primary)}
      {@const primaryTarget = targetFor(b.primary)}
      {@const primaryLatest = latestFor(b.primary)}
      {@const primaryPinned = pinnedVersions[primaryKey]}
      {@const primaryAside = primaryRel === "ahead" || (primaryRel === "same" && primaryTarget != null && primaryTarget !== (b.primary.current_version ?? ""))}
      <li class="feature-row" class:is-update={b.anyOutdated && !b.allDisabled} class:disabled={b.allDisabled} oncontextmenu={(e) => onRowContextMenu(primaryKey, e)}>
        <label class="feature-check" title={selState === "all" ? $t("component.gameDrawer.feature.deselectAll") : $t("component.gameDrawer.feature.selectAll")}>
          <input
            class="sr-only"
            type="checkbox"
            checked={selState !== "none"}
            indeterminate={selState === "some"}
            disabled={b.allDisabled || !b.anyOutdated}
            aria-label={selState === "all" ? $t("component.gameDrawer.feature.deselectAll") : $t("component.gameDrawer.feature.selectAll")}
            onchange={(e) => onToggleFeatureSelection(b, (e.target as HTMLInputElement).checked)}
          />
          <span class="check-box"></span>
        </label>
        <div class="feature-glyph" style:--feature-accent={b.accent} aria-hidden="true">
          <FeatureIcon id={b.iconId} size={20} />
        </div>
        <div class="feature-body">
          <div class="feature-head">
            <span class="feature-title">{b.title}</span>
            {#if b.statusTone === "update"}
              <span class="chip chip-update">{b.statusLabel}</span>
            {:else if b.statusTone === "success"}
              <span class="chip chip-success">{b.statusLabel}</span>
            {:else if b.statusTone === "info"}
              <span class="chip chip-info">{b.statusLabel}</span>
            {:else}
              <span class="chip chip-neutral">{b.statusLabel}</span>
            {/if}
          </div>
          <p class="feature-blurb">{b.blurb}</p>
          <div class="feature-versions">
            <span class="ver-pair">
              <span class="ver current" class:is-update={primaryRel === "outdated"}>v{b.primary.current_version ?? "?"}</span>
              {#if primaryAside}
                <button class="ver catalog-aside" onclick={() => onSetPickerOpen(primaryKey)} title={$t("component.gameDrawer.version.pickDifferent")}>
                  <span class="muted">{$t("component.gameDrawer.version.catalogAside", { version: primaryTarget ?? "" })}</span>
                </button>
              {:else}
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                <button class="ver target-btn" onclick={() => onSetPickerOpen(primaryKey)} title={$t("component.gameDrawer.version.choose")}>
                  {#if primaryTarget}
                    <span class="target">v{primaryTarget}</span>
                    {#if primaryPinned && primaryPinned !== primaryLatest}
                      <span class="chip chip-update pin-chip" title={$t("component.gameDrawer.version.pinnedTitle")}>{$t("component.gameDrawer.version.pinned")}</span>
                    {/if}
                  {:else}
                    <span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>
                  {/if}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                </button>
              {/if}
            </span>
            {#if b.records.length > 1}
              <button class="files-toggle" onclick={() => onToggleFeatureExpanded(b.feature)} aria-expanded={expanded}>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                {expanded ? $t("component.gameDrawer.files.hideMany") : $t("component.gameDrawer.files.showMany", { count: b.records.length })}
              </button>
            {:else}
              <button class="files-toggle subtle" onclick={() => onToggleFeatureExpanded(b.feature)} aria-expanded={expanded} title={filenameFromPath(b.primary.path)}>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
                {expanded ? $t("component.gameDrawer.files.hideOne") : $t("component.gameDrawer.files.showOne")}
              </button>
            {/if}
          </div>
        </div>
        <div class="feature-tools">
          <button class="feature-eye" onclick={() => onToggleFeatureDisabled(b.records)} title={b.allDisabled ? $t("component.gameDrawer.feature.reEnable") : $t("component.gameDrawer.feature.disable")} aria-label={b.allDisabled ? $t("component.gameDrawer.feature.reEnable") : $t("component.gameDrawer.feature.disable")}>
            {#if b.allDisabled}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
            {:else}
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            {/if}
          </button>
          <button class="feature-eye" onclick={(e) => openMenuFromButton(primaryKey, e)} title={$t("component.gameDrawer.feature.moreActions")} aria-label={$t("component.gameDrawer.feature.moreActions")} aria-haspopup="menu">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="5" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="12" cy="19" r="1"/></svg>
          </button>
        </div>
        {#if expanded}
          <ul class="files-list">
            {#each b.records as r (r.path)}
              {@const k = rowKey(r)}
              {@const tgt = targetFor(r)}
              {@const lat = latestFor(r)}
              {@const rel = relation(r)}
              {@const fd = disabledFamilies.includes(r.family)}
              {@const pin = pinnedVersions[k]}
              {@const fileAside = rel === "ahead" || (rel === "same" && tgt != null && tgt !== (r.current_version ?? ""))}
              <li class="file-row" class:disabled={fd}>
                <label class="file-check">
                  <input
                    class="sr-only"
                    type="checkbox"
                    checked={selected[k] ?? false}
                    disabled={fd || rel === "same" || rel === "no-target"}
                    aria-label={$t("component.gameDrawer.files.checkboxAria", { file: filenameFromPath(r.path) })}
                    onchange={(e) => onToggleFileSelection(k, (e.target as HTMLInputElement).checked)}
                  />
                  <span class="check-box"></span>
                </label>
                <div class="file-info">
                  <div class="file-top">
                    <span class="file-name mono">{filenameFromPath(r.path)}</span>
                    <span class="file-tag">{familyShort(r.family)}</span>
                  </div>
                  <div class="file-versions">
                    <span class="ver current mono" class:is-update={rel === "outdated"}>v{r.current_version ?? "?"}</span>
                    {#if fileAside}
                      <button class="ver catalog-aside small" onclick={() => onSetPickerOpen(k)} title={$t("component.gameDrawer.version.pickDifferent")}>
                        <span class="muted mono">{$t("component.gameDrawer.version.catalogAside", { version: tgt ?? "" })}</span>
                      </button>
                    {:else}
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                      <button class="ver target-btn small" onclick={() => onSetPickerOpen(k)}>
                        {#if tgt}
                          <span class="target mono">v{tgt}</span>
                          {#if pin && pin !== lat}<span class="chip chip-update pin-chip">{$t("component.gameDrawer.version.pinned")}</span>{/if}
                        {:else}
                          <span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>
                        {/if}
                        <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                      </button>
                    {/if}
                  </div>
                  <div class="file-path mono truncate" title={r.path}>{r.path}</div>
                </div>
                <div class="file-status">
                  {#if rel === "outdated"}
                    <span class="chip chip-update small-chip">{$t("component.gameDrawer.fileStatus.update")}</span>
                  {:else if rel === "ahead"}
                    <span class="chip chip-info small-chip" title={$t("component.gameDrawer.fileStatus.aheadTitle")}>{$t("component.gameDrawer.fileStatus.ahead")}</span>
                  {:else if rel === "same"}
                    <span class="chip chip-success small-chip">{$t("component.gameDrawer.fileStatus.current")}</span>
                  {:else}
                    <span class="chip chip-neutral small-chip">{$t("component.gameDrawer.status.notInCatalog")}</span>
                  {/if}
                </div>
                {#if pickerOpenFor === k}
                  <VersionPickerPopover
                    family={r.family}
                    filename={filenameFromPath(r.path)}
                    currentVersion={r.current_version}
                    latestVersion={lat}
                    pickedVersion={pin ?? null}
                    onPick={(v) => onSetPin(k, v)}
                    onClose={() => onSetPickerOpen(null)}
                  />
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
        {#if pickerOpenFor === primaryKey}
          <VersionPickerPopover
            family={b.primary.family}
            filename={filenameFromPath(b.primary.path)}
            currentVersion={b.primary.current_version}
            latestVersion={primaryLatest}
            pickedVersion={primaryPinned ?? null}
            onPick={(v) => onSetPin(primaryKey, v)}
            onClose={() => onSetPickerOpen(null)}
          />
        {/if}
      </li>
    {/each}
  </ul>
{/if}

{#if advancedRows.length > 0}
  <section class="advanced-block" class:open={advancedExpanded}>
    <button type="button" class="advanced-head" onclick={onToggleAdvanced} aria-expanded={advancedExpanded}>
      <span class="advanced-titles">
        <span class="advanced-name">
          <span class="advanced-dot" style:background={GROUP_ACCENT.advanced}></span>
          {$t("feature.advanced.title")}
          <span class="chip chip-neutral small-chip count">{advancedRows.length}</span>
        </span>
        <span class="advanced-sub">{$t("feature.advanced.blurb")}</span>
      </span>
      <span class="advanced-chevron" class:open={advancedExpanded}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
      </span>
    </button>
    {#if advancedExpanded}
      <ul class="files-list flat">
        {#each advancedRows as ar (ar.family)}
          {@const r = ar.primary}
          {@const k = rowKey(r)}
          {@const tgt = targetFor(r)}
          {@const lat = latestFor(r)}
          {@const rel = relation(r)}
          {@const fd = disabledFamilies.includes(r.family)}
          {@const pin = pinnedVersions[k]}
          <li class="file-row" class:disabled={fd}>
            <label class="file-check">
              <input
                class="sr-only"
                type="checkbox"
                checked={selected[k] ?? false}
                disabled={fd || rel === "same" || rel === "no-target"}
                aria-label={$t("component.gameDrawer.files.checkboxAria", { file: filenameFromPath(r.path) })}
                onchange={(e) => onToggleFileSelection(k, (e.target as HTMLInputElement).checked)}
              />
              <span class="check-box"></span>
            </label>
            <div class="file-info">
              <div class="file-top">
                <span class="file-name">{ar.label}</span>
                <span class="file-tag mono">{filenameFromPath(r.path)}</span>
              </div>
              <div class="file-versions">
                <span class="ver current mono" class:is-update={rel === "outdated"}>v{r.current_version ?? "?"}</span>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="arrow"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>
                <button class="ver target-btn small" onclick={() => onSetPickerOpen(k)}>
                  {#if tgt}<span class="target mono">v{tgt}</span>{:else}<span class="muted">{$t("component.gameDrawer.version.chooseVersion")}</span>{/if}
                  <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev"><polyline points="6 9 12 15 18 9"/></svg>
                </button>
              </div>
            </div>
            <div class="file-status">
              {#if rel === "outdated"}
                <span class="chip chip-update small-chip">{$t("component.gameDrawer.fileStatus.update")}</span>
              {:else if rel === "ahead"}
                <span class="chip chip-info small-chip" title={$t("component.gameDrawer.fileStatus.aheadTitle")}>{$t("component.gameDrawer.fileStatus.ahead")}</span>
              {:else if rel === "same"}
                <span class="chip chip-success small-chip">{$t("component.gameDrawer.fileStatus.current")}</span>
              {:else}
                <span class="chip chip-neutral small-chip">{$t("component.gameDrawer.status.notInCatalog")}</span>
              {/if}
            </div>
            {#if pickerOpenFor === k}
              <VersionPickerPopover
                family={r.family}
                filename={filenameFromPath(r.path)}
                currentVersion={r.current_version}
                latestVersion={lat}
                pickedVersion={pin ?? null}
                onPick={(v) => onSetPin(k, v)}
                onClose={() => onSetPickerOpen(null)}
              />
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}
{/if}

<section class="advanced-block" class:open={dlssExpanded}>
  <button type="button" class="advanced-head" onclick={onToggleDlss} aria-expanded={dlssExpanded}>
    <span class="advanced-titles">
      <span class="advanced-name">
        <span class="advanced-dot dot-nvidia"></span>
        {$t("view.drivers.dlssOverrides")}
        <span class="chip chip-neutral small-chip count">NVIDIA</span>
      </span>
      <span class="advanced-sub">{$t("component.gameDrawer.dlss.sub")}</span>
    </span>
    <span class="advanced-chevron" class:open={dlssExpanded}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
    </span>
  </button>
  {#if dlssExpanded}
    <div class="dlss-drawer-body">
      {#if dlssExeResolving}
        <p class="advanced-sub">{$t("component.gameDrawer.dlss.locating")}</p>
      {:else if dlssExe}
        <p class="advanced-sub mono truncate" title={dlssExe}>{dlssExe}</p>
        <DlssOverridePanel scope={{ scope: "per_game", executable_path: dlssExe }} driverPacked={dlssDriverPacked} />
      {:else}
        <p class="advanced-sub">{$t("component.gameDrawer.dlss.noExe")}</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .summary-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
  }
  .summary-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-2);
    background: var(--bg-cap);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .stat-num {
    font-size: var(--fs-xl);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tighter);
    font-variant-numeric: tabular-nums;
    line-height: var(--lh-tight);
  }
  .stat-num.is-update { color: var(--update); }
  .stat-num.is-accent { color: var(--accent); }
  .stat-label {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }

  .quick-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }

  .feature-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
  .feature-row {
    position: relative;
    display: grid;
    grid-template-columns: 22px 36px 1fr auto;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-4);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .feature-row:hover { background: var(--bg-card-hover); border-color: var(--border-hover); }
  .feature-row.is-update {
    border-color: color-mix(in srgb, var(--update) 40%, var(--border));
    background: color-mix(in srgb, var(--update-dim) 50%, var(--bg-card));
  }
  .feature-row.is-update::before {
    content: "";
    position: absolute;
    left: 0;
    top: var(--space-3);
    bottom: var(--space-3);
    width: 3px;
    border-radius: 0 var(--radius-full) var(--radius-full) 0;
    background: var(--update);
  }
  .feature-row.disabled { opacity: 0.55; }

  .feature-check { display: inline-flex; cursor: pointer; padding-top: 3px; }
  .check-box {
    width: 16px;
    height: 16px;
    border: 1.5px solid var(--border-strong);
    border-radius: var(--radius-xs);
    display: inline-block;
    background: var(--bg-input);
    transition: background 0.15s var(--ease), border-color 0.15s var(--ease), box-shadow 0.15s var(--ease);
    position: relative;
  }
  .feature-check input:focus-visible + .check-box,
  .file-check input:focus-visible + .check-box {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .feature-check input:checked + .check-box,
  .file-check input:checked + .check-box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .feature-check input:checked + .check-box::after,
  .file-check input:checked + .check-box::after {
    content: '';
    position: absolute;
    left: 4px;
    top: 1px;
    width: 5px;
    height: 9px;
    border: solid var(--accent-fg);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }
  .feature-check input:indeterminate + .check-box {
    background: var(--accent-dim);
    border-color: var(--accent);
  }
  .feature-check input:indeterminate + .check-box::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 6px;
    width: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
  }
  .feature-check input:disabled + .check-box,
  .file-check input:disabled + .check-box { opacity: 0.3; cursor: not-allowed; }

  .feature-glyph {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--feature-accent) 16%, var(--bg-elevated));
    border: 1px solid color-mix(in srgb, var(--feature-accent) 28%, transparent);
    color: var(--feature-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .feature-body { min-width: 0; }
  .feature-head { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
  .feature-title { font-size: var(--fs-base); font-weight: 600; color: var(--text-primary); letter-spacing: var(--letter-tight); }
  .feature-blurb { font-size: var(--fs-xs); color: var(--text-muted); margin-top: var(--space-1); line-height: var(--lh-snug); }

  .feature-versions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-2);
    flex-wrap: wrap;
  }
  .ver-pair { display: inline-flex; align-items: center; gap: var(--space-1); font-family: var(--font-mono); font-size: var(--fs-xs); color: var(--text-secondary); }
  .ver { font-variant-numeric: tabular-nums; }
  .ver.current.is-update { color: var(--update); font-weight: 600; }
  .ver-pair .arrow { color: var(--text-muted); }
  .target { color: var(--accent); font-weight: 500; }
  .muted { color: var(--text-muted); font-style: italic; }
  .target-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font: inherit;
    color: inherit;
    font-family: var(--font-mono);
  }
  .target-btn:hover { background: var(--bg-card-hover); border-color: var(--border); }
  .target-btn:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .target-btn .chev { color: var(--text-muted); }
  .target-btn.small { font-size: var(--fs-xs); padding: 1px 6px; }
  .catalog-aside {
    display: inline-flex;
    align-items: center;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font: inherit;
    color: inherit;
    font-family: var(--font-mono);
  }
  .catalog-aside:hover { background: var(--bg-card-hover); border-color: var(--border); }
  .catalog-aside:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .catalog-aside.small { font-size: var(--fs-xs); padding: 1px 6px; }
  .pin-chip { padding: 1px 6px; font-size: 9px; margin-left: 4px; }

  .files-toggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-xs);
    font-family: var(--font-sans);
  }
  .files-toggle:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .files-toggle:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .files-toggle .chev { transition: transform 0.15s var(--ease); }
  .files-toggle .chev.open { transform: rotate(180deg); }
  .files-toggle.subtle { color: var(--text-placeholder); }
  .files-toggle.subtle:hover { color: var(--text-secondary); }

  .feature-tools {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .feature-eye {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    flex-shrink: 0;
  }
  .feature-eye:hover { background: var(--bg-elevated); color: var(--text-primary); }
  .feature-eye:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  .files-list {
    grid-column: 1 / -1;
    list-style: none;
    margin: var(--space-3) 0 2px;
    padding: var(--space-2) 6px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
  }
  .files-list.flat {
    grid-column: auto;
    margin: 0;
    background: var(--bg-card);
    border-radius: var(--radius-md);
  }
  .file-row {
    position: relative;
    display: grid;
    grid-template-columns: 22px 1fr auto;
    gap: var(--space-2);
    align-items: flex-start;
    padding: 9px 10px;
    border-top: 1px solid var(--border);
  }
  .file-row:first-child { border-top: none; }
  .file-row:hover { background: var(--bg-card-hover); }
  .file-row.disabled { opacity: 0.5; }
  .file-check { display: inline-flex; cursor: pointer; padding-top: 2px; }
  .file-info { min-width: 0; }
  .file-top { display: flex; align-items: baseline; gap: var(--space-2); }
  .file-name { font-size: var(--fs-xs); font-weight: 500; color: var(--text-primary); }
  .file-tag {
    font-size: 9.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .file-versions { display: flex; align-items: center; gap: 5px; margin-top: 3px; font-size: var(--fs-2xs); color: var(--text-secondary); }
  .file-versions .arrow { color: var(--text-muted); }
  .file-path { font-size: 9.5px; color: var(--text-muted); opacity: 0.7; margin-top: 3px; }
  .file-status { padding-top: 2px; }
  .small-chip { padding: 1px 7px; font-size: 9.5px; letter-spacing: 0.04em; }

  .advanced-block {
    background: transparent;
    border: none;
    border-top: 1px solid var(--border);
    border-radius: 0;
    overflow: hidden;
    transition: background var(--dur-fast) var(--ease);
  }
  .advanced-block.open { background: transparent; }
  .advanced-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-3) 4px;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .advanced-head:hover { background: var(--bg-card-hover); border-radius: var(--radius-md); }
  .advanced-head:focus-visible { outline: none; box-shadow: var(--shadow-ring); border-radius: var(--radius-md); }
  .advanced-titles { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .advanced-name {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .advanced-dot { width: 8px; height: 8px; border-radius: 50%; box-shadow: 0 0 6px currentColor; }
  .advanced-dot.dot-nvidia { background: var(--vendor-nvidia); }
  .advanced-sub { font-size: var(--fs-2xs); color: var(--text-muted); margin-top: 2px; line-height: 1.4; }
  .advanced-chevron {
    color: var(--text-muted);
    transition: transform 0.18s var(--ease);
    display: inline-flex;
  }
  .advanced-chevron.open { transform: rotate(180deg); color: var(--text-primary); }
  .count.chip { padding: 1px 7px; }
  .dlss-drawer-body { padding: 0 4px var(--space-3); display: flex; flex-direction: column; gap: var(--space-2); }

  @container drawer (max-width: 420px) {
    .summary-row { gap: var(--space-1); }
    .summary-stat { padding: var(--space-2) var(--space-1); }
    .feature-row {
      grid-template-columns: 22px 1fr auto;
      column-gap: var(--space-2);
    }
    .feature-glyph { display: none; }
  }
</style>
