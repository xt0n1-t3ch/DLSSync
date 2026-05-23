<script lang="ts">
  import { toasts, dismissToast } from "../lib/stores";
  import { fly } from "svelte/transition";
</script>

<div class="toast-container">
  {#each $toasts as t (t.id)}
    <div
      class="toast toast-{t.kind}"
      in:fly={{ y: 16, duration: 200 }}
      out:fly={{ y: 16, duration: 150 }}
    >
      <span class="toast-msg">{t.message}</span>
      <button class="toast-close" onclick={() => dismissToast(t.id)} aria-label="Dismiss">x</button>
    </div>
  {/each}
</div>

<style>
  .toast-container { position: fixed; right: 16px; bottom: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 200; pointer-events: none; }
  .toast { pointer-events: auto; display: flex; align-items: center; gap: 12px; padding: 10px 14px; background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius-md); box-shadow: var(--shadow-md); font-size: var(--fs-sm); }
  .toast-success { border-color: var(--success); }
  .toast-warning { border-color: var(--warning); }
  .toast-danger { border-color: var(--danger); }
  .toast-msg { color: var(--text-primary); }
  .toast-close { color: var(--text-muted); font-size: 14px; line-height: 1; padding: 0 4px; cursor: pointer; background: none; border: none; }
  .toast-close:hover { color: var(--text-primary); }
</style>
