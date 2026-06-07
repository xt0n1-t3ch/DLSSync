<script lang="ts">
  type Tone = "default" | "update" | "success" | "warning" | "danger";

  let {
    count,
    tone = "default",
    collapsed = false,
    overlay = false,
    ariaLabel,
  }: {
    count: number;
    tone?: Tone;
    collapsed?: boolean;
    overlay?: boolean;
    ariaLabel?: string;
  } = $props();

  let display = $derived(count > 99 ? "99+" : String(count));
</script>

{#if count > 0}
  <span
    class="counter-pill tone-{tone}"
    class:is-collapsed={collapsed}
    class:is-overlay={overlay}
    aria-label={ariaLabel}
  >
    {display}
  </span>
{/if}

<style>
  .counter-pill {
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    font-weight: 700;
    padding: 1px 7px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    min-width: 18px;
    text-align: center;
    line-height: var(--lh-tight);
    display: inline-block;
  }
  .counter-pill.tone-update { background: var(--update-dim); color: var(--update); }
  .counter-pill.tone-success { background: var(--success-dim); color: var(--success); }
  .counter-pill.tone-warning { background: var(--warning-dim); color: var(--warning); }
  .counter-pill.tone-danger { background: var(--danger-dim); color: var(--danger); }

  .counter-pill.is-collapsed {
    position: absolute;
    top: 2px;
    right: 2px;
    padding: 0 4px;
    min-width: 14px;
  }

  .counter-pill.is-overlay {
    position: absolute;
    top: -4px;
    right: -4px;
    box-shadow: 0 0 0 2px var(--bg-base);
  }
</style>
