<script lang="ts">
  import { fly } from "svelte/transition";
  import { dockItems, applyModalOpen, currentView } from "../lib/stores";
  import { t } from "../lib/i18n/index";

  let items = $derived($dockItems);
  let count = $derived(items.length);
  let primary = $derived(items[0] ?? null);

  let fraction = $derived.by<number | null>(() => {
    const known = items.map((i) => i.fraction).filter((f): f is number => f !== null);
    if (known.length === 0) return null;
    return known.reduce((a, b) => a + b, 0) / known.length;
  });

  let headline = $derived(
    count === 0
      ? ""
      : count === 1
        ? primary!.label
        : $t("component.chrome.dock.tasksRunning", { count }),
  );
  let substage = $derived(
    count === 1 && primary ? primary.stage.replace(/_/g, " ") : $t("component.chrome.dock.inProgress"),
  );
  let failed = $derived(items.some((i) => i.stage === "failed"));

  function expand(): void {
    if (items.some((i) => i.kind === "apply")) {
      applyModalOpen.set(true);
    } else {
      currentView.set("drivers");
    }
  }
</script>

{#if count > 0}
  <aside
    class="activity-dock glass-panel"
    class:is-failed={failed}
    role="status"
    aria-live="polite"
    aria-label={$t("component.chrome.dock.backgroundActivity")}
    transition:fly={{ y: 90, duration: 240 }}
  >
    <span class="dock-dot" aria-hidden="true"></span>
    <div class="dock-body">
      <div class="dock-text">
        <span class="dock-headline">{headline}</span>
        <span class="dock-stage">{substage}</span>
      </div>
      <div class="dock-track" class:indeterminate={fraction === null}>
        {#if fraction !== null}
          <div class="dock-fill" style:width={`${Math.round(fraction * 100)}%`}></div>
        {:else}
          <div class="dock-fill dock-fill-indeterminate"></div>
        {/if}
      </div>
    </div>
    <button class="dock-expand" onclick={expand} title={$t("component.chrome.dock.showDetails")} aria-label={$t("component.chrome.dock.showDetails")}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="18 15 12 9 6 15" /></svg>
    </button>
  </aside>
{/if}

<style>
  .activity-dock {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    z-index: 90;
    width: min(560px, calc(100vw - var(--sidebar-width) - 64px));
    height: var(--dock-height);
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 14px 0 18px;
    border-radius: var(--radius-xl);
    box-shadow: var(--glass-edge), var(--shadow-lg);
  }

  .dock-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 0 var(--accent-glow);
    flex-shrink: 0;
    animation: dockPulse 1.8s var(--ease) infinite;
  }
  @keyframes dockPulse {
    0% { box-shadow: 0 0 0 0 var(--accent-glow); }
    70% { box-shadow: 0 0 0 7px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }

  .dock-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 6px; }
  .dock-text { display: flex; align-items: baseline; gap: 8px; min-width: 0; }
  .dock-headline {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dock-stage {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: capitalize;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .dock-track {
    height: 4px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .dock-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-progress), color-mix(in oklab, var(--accent-progress) 60%, #ffffff));
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent-progress) 55%, transparent);
    border-radius: inherit;
    transition: width var(--dur-normal) var(--ease);
  }
  .activity-dock.is-failed .dock-fill { background: var(--danger); box-shadow: none; }
  .activity-dock.is-failed .dock-dot { background: var(--danger); animation: none; }
  .dock-fill-indeterminate {
    width: 36%;
    animation: dockSlide 1.3s var(--ease) infinite;
  }
  @keyframes dockSlide {
    0% { transform: translateX(-120%); }
    100% { transform: translateX(320%); }
  }
  .dock-expand {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .dock-expand:hover { background: var(--bg-elevated); color: var(--text-primary); }
  .dock-expand:focus-visible { outline: none; box-shadow: var(--shadow-ring); }

  @media (prefers-reduced-motion: reduce) {
    .dock-dot, .dock-fill-indeterminate { animation: none; }
  }
</style>
