<script lang="ts">
  let { id = "advanced", size = 18, strokeWidth = 1.75 }: { id?: string; size?: number; strokeWidth?: number } = $props();
</script>

{#if id === "dlss"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="3" y="3" width="7" height="7" rx="1.4" />
    <path d="M14 5h7" />
    <path d="M14 8.5h5" />
    <path d="M3 14h18" />
    <path d="M3 18h12" />
    <path d="M17 17l2.5 2.5L23 16" />
  </svg>
{:else if id === "frame_gen"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="3" y="5" width="6" height="14" rx="1.2" />
    <rect x="10.5" y="7.5" width="4" height="9" rx="1" opacity="0.7" />
    <rect x="15.5" y="9.5" width="2.5" height="5" rx="0.8" opacity="0.5" />
    <path d="M20 12h2" />
  </svg>
{:else if id === "ray_recon"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="2.5" />
    <path d="M12 3v3" />
    <path d="M12 18v3" />
    <path d="M3 12h3" />
    <path d="M18 12h3" />
    <path d="M5.6 5.6l2.1 2.1" />
    <path d="M16.3 16.3l2.1 2.1" />
    <path d="M5.6 18.4l2.1-2.1" />
    <path d="M16.3 7.7l2.1-2.1" />
  </svg>
{:else if id === "fsr"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M4 17V7" />
    <path d="M4 7h6" />
    <path d="M4 12h5" />
    <path d="M13 7v10" />
    <path d="M13 7h4a2.5 2.5 0 0 1 0 5h-4" />
    <path d="M13 12l5 5" />
  </svg>
{:else if id === "xess"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M5 6l6 6-6 6" />
    <path d="M13 6l6 6-6 6" />
  </svg>
{:else if id === "xess_fg"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M4 7l4 5-4 5" />
    <path d="M10 7l4 5-4 5" />
    <path d="M16 7l4 5-4 5" opacity="0.6" />
  </svg>
{:else if id === "advanced"}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M4 6h10" />
    <circle cx="17" cy="6" r="2" />
    <path d="M19 6h1" />
    <path d="M4 12h3" />
    <circle cx="10" cy="12" r="2" />
    <path d="M12 12h8" />
    <path d="M4 18h10" />
    <circle cx="17" cy="18" r="2" />
    <path d="M19 18h1" />
  </svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="9" />
    <path d="M12 8v4" />
    <path d="M12 16h.01" />
  </svg>
{/if}
