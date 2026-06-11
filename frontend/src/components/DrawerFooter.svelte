<script lang="ts" module>
  import type { DllSetKey } from "../lib/labels";

  export type DrawerDllSet = {
    key: DllSetKey;
    label: string;
    count: number;
    target: string | null;
    blocked: boolean;
  };
</script>

<script lang="ts">
  import { t } from "../lib/i18n/index";
  import { STREAMLINE_OVERRIDE_NOTE } from "../lib/ux";

  let {
    selectedCount,
    aheadCount,
    isHidden,
    busy,
    streamlineSetCount,
    streamlineSetTarget,
    dllSets,
    acActive,
    acSeverity,
    acNames,
    acConfirming,
    onOpenFolder,
    onRescan,
    onToggleHidden,
    onApplyStreamlineSet,
    onApplyDllSet,
    onRequestApply,
    onCancelApplyConfirm,
  }: {
    selectedCount: number;
    aheadCount: number;
    isHidden: boolean;
    busy: boolean;
    streamlineSetCount: number;
    streamlineSetTarget: string | null;
    dllSets: DrawerDllSet[];
    acActive: boolean;
    acSeverity: "warning" | "danger";
    acNames: string;
    acConfirming: boolean;
    onOpenFolder: () => void;
    onRescan: () => void;
    onToggleHidden: () => void;
    onApplyStreamlineSet: () => void;
    onApplyDllSet: (key: DllSetKey) => void;
    onRequestApply: () => void;
    onCancelApplyConfirm: () => void;
  } = $props();

  let hideLabel = $derived(isHidden ? $t("component.gameDrawer.foot.restore") : $t("component.gameDrawer.foot.hide"));
</script>

<footer class="drawer-foot">
  <button class="btn btn-ghost foot-util" onclick={onOpenFolder} title={$t("component.gameDrawer.foot.openFolderTitle")} aria-label={$t("view.library.menu.openFolder")}>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
    <span class="foot-util-label">{$t("view.library.menu.openFolder")}</span>
  </button>
  <button class="btn btn-ghost foot-util" onclick={onRescan} title={$t("component.gameDrawer.foot.rescanTitle")} aria-label={$t("view.library.rescan")} disabled={busy}>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
    <span class="foot-util-label">{$t("view.library.rescan")}</span>
  </button>
  <button class="btn btn-ghost foot-util" onclick={onToggleHidden} title={hideLabel} aria-label={hideLabel}>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
    <span class="foot-util-label">{hideLabel}</span>
  </button>
  {#if aheadCount > 0}
    <span class="chip chip-info ahead-chip">{$t("component.gameDrawer.foot.aheadChip", { count: aheadCount })}</span>
  {/if}
  {#if streamlineSetCount > 0}
    <button
      class="btn btn-ghost foot-streamline"
      onclick={onApplyStreamlineSet}
      title={`${$t("component.gameDrawer.streamlineSet.title")} ${STREAMLINE_OVERRIDE_NOTE}`}
    >
      {streamlineSetTarget
        ? $t("component.gameDrawer.streamlineSet.labelVersion", { version: streamlineSetTarget, count: streamlineSetCount })
        : $t("component.gameDrawer.streamlineSet.label", { count: streamlineSetCount })}
    </button>
  {/if}
  {#each dllSets as set (set.key)}
    <button
      class="btn btn-ghost foot-streamline foot-dll-set"
      data-set={set.key}
      disabled={set.blocked}
      onclick={() => onApplyDllSet(set.key)}
      title={set.blocked
        ? $t("component.gameDrawer.dllSet.blockedFsr4")
        : $t("component.gameDrawer.dllSet.title", { label: set.label })}
    >
      {set.target
        ? $t("component.gameDrawer.dllSet.labelVersion", { label: set.label, version: set.target, count: set.count })
        : $t("component.gameDrawer.dllSet.label", { label: set.label, count: set.count })}
      {#if set.blocked}
        <span class="chip chip-warning set-gate-chip">{$t("component.gameDrawer.dllSet.requiresRdna4")}</span>
      {/if}
    </button>
  {/each}
  {#if acActive && selectedCount > 0}
    <span
      id="ac-apply-risk-note"
      class="ac-apply-risk"
      class:is-warning={acSeverity !== "danger"}
      class:is-danger={acSeverity === "danger"}
      role="note"
      title={acNames ? $t("component.gameDrawer.anticheat.apply.chipTitle", { names: acNames }) : ""}
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      <span>{acSeverity === "danger" ? $t("component.gameDrawer.anticheat.apply.chipBan") : $t("component.gameDrawer.anticheat.apply.chipRisk")}</span>
    </span>
  {/if}
  {#if acConfirming}
    <div class="ac-apply-confirm" role="alertdialog" aria-label={$t("component.gameDrawer.anticheat.apply.confirmAria")}>
      <p class="ac-confirm-text">
        {acNames
          ? $t("component.gameDrawer.anticheat.apply.confirmBody", { names: acNames })
          : $t("component.gameDrawer.anticheat.apply.confirmBodyGeneric")}
      </p>
      <div class="ac-confirm-actions">
        <button class="btn btn-sm btn-ghost ac-confirm-cancel" onclick={onCancelApplyConfirm}>
          {$t("component.gameDrawer.anticheat.apply.confirmCancel")}
        </button>
        <button class="btn btn-sm btn-danger ac-confirm-proceed" onclick={onRequestApply}>
          {$t("component.gameDrawer.anticheat.apply.confirmProceed")}
        </button>
      </div>
    </div>
  {/if}
  <button
    class="btn btn-primary halo is-update foot-apply"
    class:is-active={selectedCount > 0}
    class:is-ac-danger={acActive && acSeverity === "danger"}
    disabled={selectedCount === 0}
    aria-describedby={acActive && selectedCount > 0 ? "ac-apply-risk-note" : undefined}
    onclick={onRequestApply}
  >
    {acConfirming
      ? $t("component.gameDrawer.anticheat.apply.applyAnyway")
      : $t("component.gameDrawer.applySelected", { count: selectedCount })}
  </button>
</footer>

<style>
  .drawer-foot {
    position: sticky;
    bottom: 0;
    flex-shrink: 0;
    margin: 0;
    padding: var(--space-3) var(--space-4);
    background: var(--glass-2);
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    border-top: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
    z-index: 4;
  }
  .drawer-foot::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: calc(-1 * var(--space-4));
    height: var(--space-4);
    background: linear-gradient(to top, var(--bg-card), transparent);
    pointer-events: none;
  }
  .foot-util {
    flex: 0 0 auto;
    height: 40px;
    padding: 0 var(--space-3);
    gap: var(--space-2);
    justify-content: center;
  }
  .foot-util svg { margin: 0; flex-shrink: 0; }
  .foot-util-label {
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .foot-apply {
    flex: 1 1 auto;
    min-width: 150px;
    height: 40px;
    order: 9;
    justify-content: center;
  }
  .foot-apply.is-ac-danger {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--danger) 55%, transparent);
  }
  .foot-streamline {
    flex: 1 1 100%;
    height: 40px;
    order: 8;
    justify-content: center;
  }
  .foot-dll-set { gap: var(--space-2); }
  .foot-dll-set:disabled { opacity: 0.6; cursor: not-allowed; }
  .set-gate-chip { padding: 1px 7px; font-size: 9.5px; }
  .ahead-chip { order: 7; padding: 4px var(--space-3); flex: 0 0 auto; }

  .ac-apply-risk {
    order: 6;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px var(--space-3);
    border-radius: var(--radius-full);
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: var(--letter-wide);
    text-transform: uppercase;
    line-height: 1;
  }
  .ac-apply-risk svg { flex-shrink: 0; }
  .ac-apply-risk.is-warning {
    color: var(--warning);
    background: var(--warning-dim);
    border: 1px solid color-mix(in srgb, var(--warning) 45%, transparent);
  }
  .ac-apply-risk.is-danger {
    color: var(--danger);
    background: var(--danger-dim);
    border: 1px solid color-mix(in srgb, var(--danger) 50%, transparent);
  }

  .ac-apply-confirm {
    order: 5;
    flex: 1 1 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--danger-dim);
    border: 1px solid var(--danger);
    color: var(--danger);
    animation: ac-confirm-in var(--dur-fast) var(--ease);
  }
  @keyframes ac-confirm-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .ac-confirm-text {
    flex: 1 1 200px;
    min-width: 0;
    font-size: var(--fs-sm);
    line-height: var(--lh-snug);
    overflow-wrap: anywhere;
  }
  .ac-confirm-actions { display: flex; gap: var(--space-2); flex: 0 0 auto; }

  @media (prefers-reduced-motion: reduce) {
    .ac-apply-confirm { animation: none; }
  }

  @container drawer (max-width: 420px) {
    .foot-apply { flex: 1 1 100%; order: 9; }
    .foot-util-label { display: none; }
    .foot-util { width: 40px; padding: 0; }
  }
</style>
