<script lang="ts">
  let {
    checked = $bindable(false),
    label,
    disabled = false,
    id,
  }: { checked?: boolean; label?: string; disabled?: boolean; id?: string } = $props();

  function toggle(): void {
    if (!disabled) checked = !checked;
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      toggle();
    }
  }
</script>

<button
  type="button"
  {id}
  class="cb"
  class:checked
  class:disabled
  role="checkbox"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={toggle}
  onkeydown={onKey}
>
  <span class="cb-box" aria-hidden="true">
    {#if checked}
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
    {/if}
  </span>
  {#if label}<span class="cb-label">{label}</span>{/if}
</button>

<style>
  .cb {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-secondary);
    font: inherit;
    text-align: left;
  }
  .cb.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .cb-box {
    width: 18px;
    height: 18px;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm, 6px);
    border: 1.5px solid var(--border-strong, var(--border));
    background: var(--bg-input, var(--bg-elevated));
    color: var(--accent-fg, #fff);
    transition:
      background var(--dur-fast, 0.12s) var(--ease),
      border-color var(--dur-fast, 0.12s) var(--ease);
  }
  .cb:hover:not(.disabled) .cb-box {
    border-color: var(--accent);
  }
  .cb.checked .cb-box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .cb:focus-visible {
    outline: none;
  }
  .cb:focus-visible .cb-box {
    box-shadow: var(--shadow-ring, 0 0 0 3px var(--accent-dim));
  }
  .cb-label {
    font-size: 13px;
    line-height: 1.3;
    min-width: 0;
  }
</style>
