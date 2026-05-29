<script lang="ts">
  import {
    BRANDS,
    brandfetchLogoUrl,
    resolveBrandDomain,
    resolveBrandKey,
    type BrandKey,
  } from "../lib/brands";

  let {
    key = undefined,
    label = undefined,
    size = 14,
    showLabel = true,
    tone = "color",
  }: {
    key?: string | null;
    label?: string;
    size?: number;
    showLabel?: boolean;
    tone?: "mono" | "color";
  } = $props();

  let resolved = $derived<BrandKey | null>(resolveBrandKey(key));
  let brand = $derived(resolved ? BRANDS[resolved] : null);
  let remoteUrl = $derived(brand ? null : brandfetchLogoUrl(resolveBrandDomain(key), { size: size * 2 }));
  let remoteFailed = $state(false);
  let showRemote = $derived(!brand && !!remoteUrl && !remoteFailed);
  let text = $derived(label ?? brand?.label ?? key ?? "");
  let accent = $derived(brand && tone === "color" ? `var(${brand.accentVar})` : "currentColor");

  $effect(() => {
    void remoteUrl;
    remoteFailed = false;
  });
</script>

<span class="brand-mark" data-tone={tone} title={text} aria-label={text}>
  {#if brand}
    <svg
      class="brand-glyph"
      style:width={`${size}px`}
      style:height={`${size}px`}
      style:color={accent}
      viewBox={brand.viewBox}
      fill="currentColor"
      aria-hidden="true"
    >
      <path d={brand.path} />
    </svg>
  {:else if showRemote}
    <img
      class="brand-img"
      src={remoteUrl}
      alt=""
      width={size}
      height={size}
      loading="lazy"
      aria-hidden="true"
      onerror={() => (remoteFailed = true)}
    />
  {/if}
  {#if showLabel && text}
    <span class="brand-label">{text}</span>
  {/if}
</span>

<style>
  .brand-mark {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .brand-glyph {
    flex-shrink: 0;
    display: block;
  }
  .brand-img {
    flex-shrink: 0;
    display: block;
    object-fit: contain;
    border-radius: var(--radius-xs, 3px);
  }
  .brand-label {
    line-height: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
