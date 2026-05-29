<script lang="ts" generics="V extends string | null">
  type Opt = { value: V; label: string; disabled?: boolean };

  let {
    value = $bindable(),
    options,
    placeholder = "Select…",
    disabled = false,
    ariaLabel,
  }: {
    value: V;
    options: Opt[];
    placeholder?: string;
    disabled?: boolean;
    ariaLabel?: string;
  } = $props();

  let open = $state(false);
  let activeIndex = $state(-1);
  let root = $state<HTMLDivElement>();

  let selected = $derived(options.find((o) => o.value === value) ?? null);

  function openMenu(): void {
    if (disabled) return;
    open = true;
    activeIndex = options.findIndex((o) => o.value === value && !o.disabled);
  }
  function close(): void {
    open = false;
    activeIndex = -1;
  }
  function choose(o: Opt): void {
    if (o.disabled) return;
    value = o.value;
    close();
  }
  function move(delta: number): void {
    const n = options.length;
    if (n === 0) return;
    let i = activeIndex;
    for (let step = 0; step < n; step++) {
      i = (i + delta + n) % n;
      if (!options[i].disabled) {
        activeIndex = i;
        break;
      }
    }
  }
  function onTriggerKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      close();
      return;
    }
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        openMenu();
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (activeIndex >= 0) choose(options[activeIndex]);
    }
  }

  $effect(() => {
    if (!open) return;
    function onDocPointer(e: PointerEvent): void {
      if (root && !root.contains(e.target as Node)) close();
    }
    document.addEventListener("pointerdown", onDocPointer, true);
    return () => document.removeEventListener("pointerdown", onDocPointer, true);
  });
</script>

<div class="sel" class:open class:disabled bind:this={root}>
  <button
    type="button"
    class="sel-trigger"
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel}
    onclick={() => (open ? close() : openMenu())}
    onkeydown={onTriggerKey}
  >
    <span class="sel-value" class:is-placeholder={!selected}>{selected ? selected.label : placeholder}</span>
    <svg class="sel-chev" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9" /></svg>
  </button>
  {#if open}
    <div class="sel-menu glass-dialog" role="listbox">
      {#each options as o, i (String(o.value))}
        <button
          type="button"
          role="option"
          aria-selected={o.value === value}
          class="sel-opt"
          class:active={i === activeIndex}
          class:chosen={o.value === value}
          disabled={o.disabled}
          onclick={() => choose(o)}
          onpointerenter={() => !o.disabled && (activeIndex = i)}
        >
          <span class="sel-opt-label">{o.label}</span>
          {#if o.value === value}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: block;
    width: 100%;
    min-width: 0;
  }
  .sel-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--radius-md, 8px);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition:
      border-color var(--dur-fast, 0.12s) var(--ease),
      background var(--dur-fast, 0.12s) var(--ease);
  }
  .sel-trigger:hover:not(:disabled) {
    border-color: var(--border-hover, var(--accent));
  }
  .sel.open .sel-trigger {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .sel-trigger:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: var(--shadow-ring, 0 0 0 3px var(--accent-dim));
  }
  .sel-trigger:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .sel-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-value.is-placeholder {
    color: var(--text-muted);
  }
  .sel-chev {
    flex: 0 0 auto;
    color: var(--text-muted);
    transition: transform 0.15s var(--ease);
  }
  .sel.open .sel-chev {
    transform: rotate(180deg);
    color: var(--accent);
  }
  .sel-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 240;
    list-style: none;
    margin: 0;
    padding: 4px 4px 4px 6px;
    max-height: 280px;
    overflow-y: auto;
    border-radius: var(--radius-md, 8px);
  }
  .sel-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 8px 9px;
    border: none;
    border-radius: var(--radius-sm, 6px);
    background: none;
    font: inherit;
    font-size: 13px;
    text-align: left;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .sel-opt-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-opt.active {
    background: var(--bg-card-hover, var(--bg-elevated));
    color: var(--text-primary);
  }
  .sel-opt.chosen {
    color: var(--accent);
    font-weight: 600;
  }
  .sel-opt:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
