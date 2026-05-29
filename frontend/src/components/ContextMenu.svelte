<script lang="ts" module>
  export type ContextMenuAction = "open_folder" | "scan" | "pin" | "hide";
  export interface ContextMenuItem {
    action: ContextMenuAction;
    label: string;
  }
</script>

<script lang="ts">
  import { tick } from "svelte";

  let { x, y, items, onSelect, onClose }: {
    x: number;
    y: number;
    items: ContextMenuItem[];
    onSelect: (action: ContextMenuAction) => void;
    onClose: () => void;
  } = $props();

  const ICONS: Record<ContextMenuAction, string> = {
    open_folder: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z",
    scan: "M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15",
    pin: "M12 17v5M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z",
    hide: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM4.93 4.93l14.14 14.14",
  };

  let menuEl = $state<HTMLElement | undefined>(undefined);
  let clamped = $state<{ left: number; top: number } | null>(null);
  let pos = $derived(clamped ?? { left: x, top: y });

  $effect(() => {
    void tick().then(() => {
      if (!menuEl) return;
      const rect = menuEl.getBoundingClientRect();
      const margin = 8;
      let left = x;
      let top = y;
      if (left + rect.width > window.innerWidth - margin) {
        left = Math.max(margin, window.innerWidth - rect.width - margin);
      }
      if (top + rect.height > window.innerHeight - margin) {
        top = Math.max(margin, window.innerHeight - rect.height - margin);
      }
      clamped = { left, top };
      const first = menuEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    });
  });

  function buttons(): HTMLElement[] {
    if (!menuEl) return [];
    return Array.from(menuEl.querySelectorAll<HTMLElement>('[role="menuitem"]'));
  }

  function moveFocus(delta: number): void {
    const list = buttons();
    if (list.length === 0) return;
    const idx = list.findIndex((b) => b === document.activeElement);
    const next = (idx + delta + list.length) % list.length;
    list[next].focus();
  }

  function onKey(e: KeyboardEvent): void {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        moveFocus(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveFocus(-1);
        break;
      case "Home":
        e.preventDefault();
        buttons()[0]?.focus();
        break;
      case "End": {
        e.preventDefault();
        const list = buttons();
        list[list.length - 1]?.focus();
        break;
      }
      case "Escape":
        e.preventDefault();
        onClose();
        break;
    }
  }

  function choose(action: ContextMenuAction): void {
    onSelect(action);
    onClose();
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (menuEl && !menuEl.contains(e.target as Node)) onClose();
  }}
  onresize={onClose}
/>

<div
  class="context-menu glass-panel"
  bind:this={menuEl}
  role="menu"
  tabindex="-1"
  aria-label="Game actions"
  style:left="{pos.left}px"
  style:top="{pos.top}px"
  onkeydown={onKey}
>
  {#each items as item (item.action)}
    <button
      class="ctx-item press"
      class:is-danger={item.action === "hide"}
      role="menuitem"
      tabindex="-1"
      onclick={() => choose(item.action)}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d={ICONS[item.action]} />
      </svg>
      <span>{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 220;
    min-width: 168px;
    padding: 5px;
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    font-weight: 500;
    text-align: left;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .ctx-item svg { flex-shrink: 0; color: var(--text-muted); }
  .ctx-item:hover,
  .ctx-item:focus-visible {
    outline: none;
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .ctx-item:hover svg,
  .ctx-item:focus-visible svg { color: var(--accent); }
  .ctx-item.is-danger:hover,
  .ctx-item.is-danger:focus-visible {
    background: var(--danger-dim);
    color: var(--danger);
  }
  .ctx-item.is-danger:hover svg,
  .ctx-item.is-danger:focus-visible svg { color: var(--danger); }
</style>
