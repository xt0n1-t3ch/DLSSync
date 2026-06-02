<script lang="ts">
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { locale, setLocale, LOCALES, LOCALE_LABELS, t, type Locale } from "../lib/i18n/index";
  import { settings, persistSettings } from "../lib/stores";

  let { open, onClose }: { open: boolean; onClose: () => void } = $props();

  let panelEl: HTMLDivElement | undefined = $state();
  let activeIndex = $state(0);

  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function choose(loc: Locale): void {
    setLocale(loc);
    if ($settings) {
      void persistSettings({
        ...$settings,
        ui_prefs: { ...$settings.ui_prefs, language: loc },
      });
    }
    onClose();
  }

  function move(delta: number): void {
    const n = LOCALES.length;
    activeIndex = (activeIndex + delta + n) % n;
  }

  $effect(() => {
    if (!open) return;
    activeIndex = Math.max(0, LOCALES.indexOf($locale));
    void tick().then(() => panelEl?.focus());
  });

  onMount(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (!open) return;
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        move(1);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        move(-1);
      } else if (e.key === "Home") {
        e.preventDefault();
        activeIndex = 0;
      } else if (e.key === "End") {
        e.preventDefault();
        activeIndex = LOCALES.length - 1;
      } else if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        choose(LOCALES[activeIndex]);
      }
    };
    const onClickOutside = (e: MouseEvent): void => {
      if (!open) return;
      const target = e.target as Node | null;
      if (target instanceof Element && target.closest("[data-language-toggle]")) return;
      if (panelEl && target && !panelEl.contains(target)) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onClickOutside);
    return () => {
      window.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onClickOutside);
    };
  });
</script>

{#if open}
  <div
    class="lang-menu glass-dialog"
    role="listbox"
    aria-label={$t("language.switcherAria")}
    tabindex="-1"
    bind:this={panelEl}
    transition:fly={{ y: 8, duration: reduced ? 0 : 160 }}
  >
    {#each LOCALES as loc, i (loc)}
      <button
        type="button"
        role="option"
        aria-selected={loc === $locale}
        class="lang-opt"
        class:active={i === activeIndex}
        class:chosen={loc === $locale}
        onclick={() => choose(loc)}
        onpointerenter={() => (activeIndex = i)}
      >
        <span class="lang-opt-name">{LOCALE_LABELS[loc]}</span>
        <span class="lang-opt-code">{loc.toUpperCase()}</span>
        {#if loc === $locale}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
        {/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .lang-menu {
    position: fixed;
    left: 12px;
    bottom: 64px;
    width: 208px;
    max-width: calc(100vw - 24px);
    z-index: 240;
    --edge-color: var(--accent);
    list-style: none;
    margin: 0;
    padding: 4px 4px 4px 6px;
    border-radius: var(--radius-md, 8px);
    box-shadow: var(--shadow-lg);
  }
  .lang-opt {
    display: flex;
    align-items: center;
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
  .lang-opt-name {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lang-opt-code {
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }
  .lang-opt.active {
    background: var(--bg-card-hover, var(--bg-elevated));
    color: var(--text-primary);
  }
  .lang-opt.chosen {
    color: var(--accent);
    font-weight: 600;
  }
  .lang-opt.chosen .lang-opt-code {
    color: var(--accent);
  }
  .lang-opt:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
</style>
