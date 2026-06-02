<script lang="ts">
  import { toasts, dismissToast, type Toast } from "../lib/stores";
  import { t as tr } from "../lib/i18n/index";
  import { fly } from "svelte/transition";

  function iconPaths(kind: Toast["kind"]): string {
    switch (kind) {
      case "success":
        return "M22 11.08V12a10 10 0 1 1-5.93-9.14|M22 4 12 14.01 9 11.01";
      case "warning":
        return "M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z|M12 9v4|M12 17h.01";
      case "danger":
        return "M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z|M12 9v4|M12 17h.01";
      default:
        return "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z|M12 16v-4|M12 8h.01";
    }
  }
</script>

<div class="toast-container">
  {#each $toasts as t (t.id)}
    <div
      class="toast toast-{t.kind}"
      in:fly={{ y: 16, duration: 200 }}
      out:fly={{ y: 16, duration: 150 }}
    >
      <span class="toast-icon" data-kind={t.kind} aria-hidden="true">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          {#each iconPaths(t.kind).split("|") as d}
            <path d={d} />
          {/each}
        </svg>
      </span>
      <span class="toast-msg">{t.message}</span>
      {#if t.action}
        <button class="toast-action" onclick={() => t.action?.run()}>{t.action.label}</button>
      {/if}
      <button class="toast-close" onclick={() => dismissToast(t.id)} aria-label={$tr("common.dismiss")}>x</button>
      <span class="toast-progress" data-kind={t.kind} style:--toast-ttl="{t.ttlMs}ms" aria-hidden="true"></span>
    </div>
  {/each}
</div>

<style>
  .toast-container { position: fixed; right: 16px; bottom: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 200; pointer-events: none; }
  .toast { position: relative; pointer-events: auto; display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--radius-md); box-shadow: var(--shadow-md); font-size: var(--fs-sm); overflow: hidden; }
  .toast-success { border-color: var(--success); }
  .toast-warning { border-color: var(--warning); }
  .toast-danger { border-color: var(--danger); }
  .toast-info { border-color: var(--info); }
  .toast-icon { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .toast-icon[data-kind="success"] { color: var(--success); }
  .toast-icon[data-kind="warning"] { color: var(--warning); }
  .toast-icon[data-kind="danger"] { color: var(--danger); }
  .toast-icon[data-kind="info"] { color: var(--info); }
  .toast-msg { color: var(--text-primary); }
  .toast-action {
    color: var(--accent);
    font-size: var(--fs-xs);
    font-weight: 600;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    background: var(--accent-dim);
    border: none;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease);
  }
  .toast-action:hover { background: var(--accent-glow, var(--accent-dim)); color: var(--accent-hover); }
  .toast-action:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .toast-close { color: var(--text-muted); font-size: 14px; line-height: 1; padding: 0 4px; cursor: pointer; background: none; border: none; }
  .toast-close:hover { color: var(--text-primary); }
  .toast-progress {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    width: 100%;
    transform-origin: left center;
    border-radius: var(--radius-full);
    animation: toast-drain var(--toast-ttl, 4000ms) linear forwards;
  }
  .toast-progress[data-kind="success"] { background: var(--success); }
  .toast-progress[data-kind="warning"] { background: var(--warning); }
  .toast-progress[data-kind="danger"] { background: var(--danger); }
  .toast-progress[data-kind="info"] { background: var(--info); }
  @keyframes toast-drain { from { transform: scaleX(1); } to { transform: scaleX(0); } }
  @media (prefers-reduced-motion: reduce) {
    .toast-progress { animation: none; transform: scaleX(1); opacity: 0.5; }
  }
</style>
