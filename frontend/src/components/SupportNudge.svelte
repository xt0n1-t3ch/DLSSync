<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Heart from "@lucide/svelte/icons/heart";
  import Star from "@lucide/svelte/icons/star";
  import Share2 from "@lucide/svelte/icons/share-2";
  import NexusLogo from "./NexusLogo.svelte";
  import X from "@lucide/svelte/icons/x";
  import { get } from "svelte/store";
  import { supportNudgeVisible, dismissNudge, dontShowAgain, shareDlssync } from "../lib/community";
  import { showSourceLinks } from "../lib/distribution";
  import { EXTERNAL_URLS } from "../lib/ux";
  import { showToast, updateBannerActive } from "../lib/stores";
  import { t, locale, translate } from "../lib/i18n/index";

  // Yield the bottom-left corner to the update banner when both want it.
  let visible = $derived($supportNudgeVisible && !$updateBannerActive);

  // Preview affordance (mirrors UpdateBanner's ?fakeUpdate): open with ?supportNudge=1.
  onMount(() => {
    try {
      if (new URLSearchParams(window.location.search).get("supportNudge") === "1") {
        supportNudgeVisible.set(true);
      }
    } catch {
      /* no query string available */
    }
  });

  async function openExternal(url: string): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch {
      window.open(url, "_blank");
    }
  }

  async function star(): Promise<void> {
    await openExternal(EXTERNAL_URLS.homepage);
    dismissNudge();
  }

  async function endorse(): Promise<void> {
    await openExternal(EXTERNAL_URLS.nexusMod);
    dismissNudge();
  }

  async function share(): Promise<void> {
    const result = await shareDlssync();
    if (result === "copied") showToast("success", translate(get(locale), "common.shareCopied"));
    else if (result === "failed") showToast("warning", translate(get(locale), "common.shareFailed"));
    dismissNudge();
  }
</script>

{#if visible}
  <div
    class="support-card"
    role="region"
    aria-label={$t("component.support.regionAria")}
    aria-live="polite"
    in:fly={{ y: 20, duration: 340, easing: cubicOut }}
    out:fly={{ y: 20, duration: 180 }}
  >
    <span class="edge" aria-hidden="true"></span>
    <button class="close" onclick={dismissNudge} aria-label={$t("common.dismiss")}><X size={13} strokeWidth={2.4} /></button>
    <div class="head">
      <span class="medallion" aria-hidden="true"><Heart size={15} fill="currentColor" strokeWidth={2} /></span>
      <div class="head-text">
        <p class="title">{$t("component.support.title")}</p>
        <p class="sub">{$t("component.support.sub")}</p>
      </div>
    </div>
    <div class="actions">
      {#if showSourceLinks}
      <button class="act is-star" onclick={star} title={$t("component.support.starTitle")}>
        <Star size={15} fill="currentColor" strokeWidth={2} /> {$t("component.support.star")}
      </button>
      {/if}
      <button class="act is-endorse" onclick={endorse} title={$t("component.support.endorseTitle")}>
        <NexusLogo size={15} /> {$t("component.support.endorse")}
      </button>
      <button class="act is-share" onclick={share} title={$t("component.support.shareTitle")}>
        <Share2 size={15} strokeWidth={2.2} /> {$t("component.support.share")}
      </button>
    </div>
    <button class="dont" onclick={() => void dontShowAgain()}>{$t("component.support.dontShowAgain")}</button>
  </div>
{/if}

<style>
  .support-card {
    position: fixed;
    bottom: 18px;
    left: 18px;
    z-index: 150;
    width: min(322px, calc(100vw - 36px));
    padding: 13px 15px 10px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .edge {
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: linear-gradient(180deg, var(--gh-star), var(--heart) 50%, var(--nexus));
    background-size: 100% 220%;
    animation: edge-shimmer 5s ease-in-out infinite alternate;
  }
  @keyframes edge-shimmer {
    from { background-position: 0 0; }
    to { background-position: 0 100%; }
  }
  .close {
    position: absolute;
    top: 7px;
    right: 7px;
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .close:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .head {
    display: grid;
    grid-template-columns: 30px 1fr;
    gap: 11px;
    align-items: center;
    padding-left: 3px;
    padding-right: 16px;
    margin-bottom: 11px;
  }
  .medallion {
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    color: var(--heart);
    background: var(--heart-dim);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--heart) 32%, transparent);
    animation: medallion-pulse 3.4s ease-in-out infinite;
  }
  @keyframes medallion-pulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.07); }
  }
  .head-text { min-width: 0; }
  .title {
    font-size: var(--fs-sm);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
    margin: 0 0 1px;
  }
  .sub {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    line-height: var(--lh-snug);
    margin: 0;
  }
  .actions {
    display: flex;
    gap: 7px;
    padding-left: 3px;
  }
  .act {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 6px;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition:
      color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      transform var(--dur-fast) var(--ease);
  }
  .act:hover { transform: translateY(-1px); color: var(--text-primary); }
  .act:active { transform: translateY(0) scale(0.97); }
  .act :global(svg) { flex-shrink: 0; }
  .act.is-star :global(svg) { color: var(--gh-star); }
  .act.is-star:hover { border-color: var(--gh-star); background: color-mix(in oklab, var(--gh-star) 12%, var(--bg-elevated)); }
  .act.is-endorse :global(svg) { color: var(--nexus); }
  .act.is-endorse:hover { border-color: var(--nexus); background: color-mix(in oklab, var(--nexus) 12%, var(--bg-elevated)); }
  .act.is-share :global(svg) { color: var(--accent); }
  .act.is-share:hover { border-color: var(--accent); background: color-mix(in oklab, var(--accent) 12%, var(--bg-elevated)); }
  .dont {
    margin: 8px 0 0 3px;
    padding: 2px 0;
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: color var(--dur-fast) var(--ease);
  }
  .dont:hover { color: var(--text-secondary); text-decoration: underline; }

  @media (max-width: 560px) {
    .support-card { width: calc(100vw - 24px); left: 12px; right: 12px; bottom: 12px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .edge, .medallion { animation: none; }
  }
</style>
