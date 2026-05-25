<script lang="ts">
  import type { Snippet } from "svelte";

  type Variant = "update" | "danger" | "success" | "neutral";

  let {
    active = false,
    variant = "update",
    tag = "div",
    children,
  }: {
    active?: boolean;
    variant?: Variant;
    tag?: "div" | "span" | "section";
    children?: Snippet;
  } = $props();
</script>

{#if tag === "span"}
  <span class="halo is-{variant}" class:is-active={active}>
    {@render children?.()}
  </span>
{:else if tag === "section"}
  <section class="halo is-{variant}" class:is-active={active}>
    {@render children?.()}
  </section>
{:else}
  <div class="halo is-{variant}" class:is-active={active}>
    {@render children?.()}
  </div>
{/if}
