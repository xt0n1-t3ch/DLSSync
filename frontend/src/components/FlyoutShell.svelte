<script lang="ts">
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";

  let {
    onClose,
    ariaLabel,
    width = "720px",
    zIndex = 220,
    backdropOpacity = 0.5,
    backdropBlur = 0,
    accent,
    onEscape,
    children,
  }: {
    onClose: () => void;
    ariaLabel: string;
    width?: string;
    zIndex?: number;
    backdropOpacity?: number;
    backdropBlur?: number;
    accent?: string;
    onEscape?: (e: KeyboardEvent) => void;
    children: Snippet;
  } = $props();

  function handleKey(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    if (onEscape) {
      onEscape(e);
    } else {
      onClose();
    }
  }
</script>

<div
  class="flyout-backdrop"
  style:--flyout-backdrop-alpha={backdropOpacity}
  style:--flyout-backdrop-blur="{backdropBlur}px"
  style:z-index={zIndex}
  role="presentation"
  onclick={onClose}
  onkeydown={handleKey}
  tabindex="-1"
></div>

<div
  class="flyout glass-dialog"
  transition:fly={{ y: -8, duration: 160 }}
  style:--edge-color={accent}
  style:--flyout-width={width}
  style:z-index={zIndex + 1}
  role="dialog"
  aria-label={ariaLabel}
  tabindex="-1"
  onkeydown={handleKey}
>
  {@render children()}
</div>

<style>
  .flyout-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, var(--flyout-backdrop-alpha, 0.5));
    backdrop-filter: blur(var(--flyout-backdrop-blur, 0));
  }
  .flyout {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(var(--flyout-width, 720px), 92vw);
    max-height: 84vh;
    display: flex;
    flex-direction: column;
  }
</style>
