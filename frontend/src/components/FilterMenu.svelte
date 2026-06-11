<script lang="ts">
  import { tick } from "svelte";
  import { fly } from "svelte/transition";

  type Option = { id: string; label: string; count?: number; tone?: "danger" | null };

  let {
    label,
    options,
    selectedId,
    onSelect,
  }: {
    label: string;
    options: Option[];
    selectedId: string;
    onSelect: (id: string) => void;
  } = $props();

  let open = $state(false);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let panelEl: HTMLDivElement | undefined = $state();
  let activeIndex = $state(0);

  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let selected = $derived(options.find((o) => o.id === selectedId) ?? options[0]);
  let activeLabel = $derived(selected?.label ?? "");
  let activeDescendant = $derived(
    options[activeIndex] ? `filter-opt-${options[activeIndex].id}` : undefined,
  );

  function openMenu(): void {
    open = true;
    activeIndex = Math.max(0, options.findIndex((o) => o.id === selectedId));
    void tick().then(() => panelEl?.focus());
  }

  function closeMenu(returnFocus = true): void {
    open = false;
    if (returnFocus) void tick().then(() => triggerEl?.focus());
  }

  function toggle(): void {
    if (open) closeMenu();
    else openMenu();
  }

  function move(delta: number): void {
    const n = options.length;
    if (n === 0) return;
    activeIndex = (activeIndex + delta + n) % n;
  }

  function choose(id: string): void {
    onSelect(id);
    closeMenu();
  }

  function onTriggerKey(e: KeyboardEvent): void {
    if (open) return;
    if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openMenu();
    }
  }

  function onPanelKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      closeMenu();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "Home") {
      e.preventDefault();
      activeIndex = 0;
    } else if (e.key === "End") {
      e.preventDefault();
      activeIndex = options.length - 1;
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      const opt = options[activeIndex];
      if (opt) choose(opt.id);
    } else if (e.key === "Tab") {
      closeMenu(false);
    }
  }

  function onWindowPointer(e: MouseEvent): void {
    if (!open) return;
    const target = e.target as Node | null;
    if (triggerEl && target && triggerEl.contains(target)) return;
    if (panelEl && target && !panelEl.contains(target)) closeMenu(false);
  }
</script>

<svelte:window onmousedown={onWindowPointer} />

<div class="filter-menu">
  <button
    class="filter-menu-trigger"
    type="button"
    bind:this={triggerEl}
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={toggle}
    onkeydown={onTriggerKey}
  >
    <span class="filter-menu-label">{label}</span>
    <span class="filter-menu-value">{activeLabel}</span>
    <svg class="filter-menu-chevron" class:is-open={open} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9" /></svg>
  </button>

  {#if open}
    <div
      class="filter-menu-popover surface"
      role="listbox"
      aria-label={label}
      aria-activedescendant={activeDescendant}
      tabindex="-1"
      bind:this={panelEl}
      onkeydown={onPanelKey}
      transition:fly={{ y: 6, duration: reduced ? 0 : 150 }}
    >
      {#each options as opt, i (opt.id)}
        <button
          class="filter-menu-option"
          id="filter-opt-{opt.id}"
          class:active={i === activeIndex}
          class:chosen={opt.id === selectedId}
          class:tone-danger={opt.tone === "danger"}
          type="button"
          role="option"
          aria-selected={opt.id === selectedId}
          onclick={() => choose(opt.id)}
          onpointerenter={() => (activeIndex = i)}
        >
          <svg class="filter-menu-check" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
          <span class="filter-menu-option-label">{opt.label}</span>
          {#if opt.count !== undefined}
            <span class="filter-menu-count">{opt.count}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .filter-menu {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }
  .filter-menu-trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    max-width: 220px;
    padding: 0 10px 0 12px;
    border-radius: var(--radius-full);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .filter-menu-trigger:hover {
    background: var(--bg-elevated);
    border-color: var(--border-hover);
    color: var(--text-primary);
  }
  .filter-menu-trigger[aria-expanded="true"] {
    background: var(--accent-dim);
    border-color: var(--accent);
    color: var(--accent);
  }
  .filter-menu-trigger:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .filter-menu-label {
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .filter-menu-trigger[aria-expanded="true"] .filter-menu-label {
    color: inherit;
  }
  .filter-menu-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-primary);
  }
  .filter-menu-trigger[aria-expanded="true"] .filter-menu-value {
    color: inherit;
  }
  .filter-menu-chevron {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform var(--dur-fast) var(--ease);
  }
  .filter-menu-trigger:hover .filter-menu-chevron,
  .filter-menu-trigger[aria-expanded="true"] .filter-menu-chevron {
    color: inherit;
  }
  .filter-menu-chevron.is-open {
    transform: rotate(180deg);
  }

  .filter-menu-popover {
    position: absolute;
    top: calc(100% + 6px);
    inset-inline-start: 0;
    z-index: 60;
    min-width: max(100%, 13rem);
    max-width: min(20rem, 90vw);
    padding: 4px;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .filter-menu-option {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .filter-menu-check {
    flex-shrink: 0;
    opacity: 0;
    color: var(--accent);
  }
  .filter-menu-option.chosen .filter-menu-check {
    opacity: 1;
  }
  .filter-menu-option-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .filter-menu-count {
    flex-shrink: 0;
    font-size: var(--fs-2xs);
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .filter-menu-option.active {
    background: var(--bg-card-hover);
    color: var(--text-primary);
  }
  .filter-menu-option.chosen {
    color: var(--accent);
    font-weight: 600;
  }
  .filter-menu-option.chosen .filter-menu-count {
    background: var(--accent-glow);
    color: var(--accent);
  }
  .filter-menu-option.tone-danger.chosen {
    color: var(--danger);
  }
  .filter-menu-option.tone-danger.chosen .filter-menu-check {
    color: var(--danger);
  }
  .filter-menu-option.tone-danger.chosen .filter-menu-count {
    background: var(--danger-glow);
    color: var(--danger);
  }
  .filter-menu-option:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
</style>
