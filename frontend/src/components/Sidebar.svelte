<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import {
    currentView,
    settings,
    persistSettings,
    sidebarCounts,
    languageMenuOpen,
  } from "../lib/stores";
  import { t, locale, LOCALE_LABELS } from "../lib/i18n/index";

  const STAGGER_STEP_MS = 28;
  const prefersReducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  function labelStagger(index: number): { x: number; duration: number; delay: number } {
    return {
      x: -8,
      duration: prefersReducedMotion ? 0 : 180,
      delay: prefersReducedMotion ? 0 : index * STAGGER_STEP_MS,
    };
  }

  let appVersion = $state("dev");
  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      appVersion = await getVersion();
    } catch {
      appVersion = "dev";
    }
  });

  type NavItem = { id: string; icon: string; counterKey?: "library" | "backups" };

  const librarySection: NavItem[] = [
    { id: "library", icon: "library", counterKey: "library" },
    { id: "catalog", icon: "catalog" },
    { id: "drivers", icon: "drivers" },
    { id: "backups", icon: "backups", counterKey: "backups" },
  ];

  const settingsSection: NavItem[] = [
    { id: "settings", icon: "settings" },
    { id: "about", icon: "about" },
  ];

  function switchView(id: string): void {
    currentView.set(id);
  }

  let collapsed = $derived($settings?.ui_prefs.sidebar_collapsed ?? false);

  function counterValue(item: NavItem): number {
    if (!item.counterKey) return 0;
    return $sidebarCounts[item.counterKey];
  }

  async function toggleCollapsed(): Promise<void> {
    if (!$settings) return;
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, sidebar_collapsed: !collapsed },
    });
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-brand" data-tauri-drag-region>
    <span class="brand-pill" aria-hidden="true">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 12a9 9 0 1 0 3-6.7"/>
        <polyline points="3 4 3 9 8 9"/>
      </svg>
    </span>
    {#if !collapsed}
      <span class="brand-wrap">
        <span class="brand-name">DLSSync</span>
        <span class="brand-version mono">v{appVersion}</span>
      </span>
    {/if}
  </div>

  <nav class="sidebar-nav">
    {#if !collapsed}<div class="nav-label">{$t("component.chrome.sidebar.libraryGroup")}</div>{/if}
    {#each librarySection as item, i}
      {@const count = counterValue(item)}
      <button class="nav-pill" class:active={$currentView === item.id} title={$t("view." + item.id + ".title")} onclick={() => switchView(item.id)}>
        {#if item.icon === "library"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/></svg>
        {:else if item.icon === "catalog"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>
        {:else if item.icon === "backups"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
        {:else if item.icon === "drivers"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></svg>
        {/if}
        {#if !collapsed}<span class="nav-label-text" in:fly={labelStagger(i)}>{$t("view." + item.id + ".title")}</span>{/if}
        {#if count > 0}
          <span class="nav-counter" class:is-collapsed={collapsed} class:is-update={item.counterKey === "library"} aria-label={item.counterKey === "library" ? $t("component.chrome.sidebar.outdatedCount", { count }) : $t("component.chrome.sidebar.restorableCount", { count })}>
            {count > 99 ? "99+" : count}
          </span>
        {/if}
      </button>
    {/each}

    {#if !collapsed}<div class="nav-label">{$t("component.chrome.sidebar.generalGroup")}</div>{/if}
    {#each settingsSection as item, i}
      <button class="nav-pill" class:active={$currentView === item.id} title={$t("view." + item.id + ".title")} onclick={() => switchView(item.id)}>
        {#if item.icon === "settings"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
        {/if}
        {#if !collapsed}<span class="nav-label-text" in:fly={labelStagger(librarySection.length + i)}>{$t("view." + item.id + ".title")}</span>{/if}
      </button>
    {/each}
  </nav>

  <button
    class="nav-pill lang-switcher"
    class:active={$languageMenuOpen}
    data-language-toggle
    aria-haspopup="listbox"
    aria-expanded={$languageMenuOpen}
    aria-label={$t("language.currentAria", { name: LOCALE_LABELS[$locale] })}
    title={$t("language.label")}
    onclick={() => languageMenuOpen.update((v) => !v)}
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
    {#if !collapsed}<span class="nav-label-text" in:fly={labelStagger(librarySection.length + settingsSection.length)}>{$t("language.label")}</span>{/if}
    <span class="lang-code nav-counter" class:is-collapsed={collapsed}>{$locale.toUpperCase()}</span>
  </button>

  <button class="sidebar-toggle" onclick={toggleCollapsed} title={collapsed ? $t("component.chrome.sidebar.expand") : $t("component.chrome.sidebar.collapse")} aria-label={collapsed ? $t("component.chrome.sidebar.expand") : $t("component.chrome.sidebar.collapse")}>
    {#if collapsed}
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
    {:else}
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
      <span class="toggle-label">{$t("component.chrome.sidebar.collapseLabel")}</span>
    {/if}
  </button>
</aside>

<style>
  .sidebar {
    position: relative;
    flex-shrink: 0;
    width: var(--sidebar-width);
    height: 100%;
    background: var(--glass-1);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-right: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    display: flex;
    flex-direction: column;
    z-index: 2;
    transition: width var(--dur-normal) var(--ease);
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .sidebar { background: var(--glass-fallback); }
  }
  .sidebar.collapsed {
    width: var(--sidebar-width-collapsed);
  }
  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: var(--topbar-height);
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  .sidebar.collapsed .sidebar-brand { padding: 0; justify-content: center; }
  .brand-pill {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-lg);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: inset 0 0 0 1px var(--accent-ring);
  }
  .brand-wrap { display: flex; flex-direction: column; gap: 0; line-height: var(--lh-tight); min-width: 0; }
  .brand-name {
    font-size: var(--fs-lg);
    font-weight: 700;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
  }
  .brand-version { font-size: var(--fs-2xs); color: var(--text-muted); margin-top: 2px; font-variant-numeric: tabular-nums; }
  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4);
    overflow-y: auto;
  }
  .sidebar.collapsed .sidebar-nav { padding: var(--space-4) var(--space-2); align-items: center; }
  .nav-label {
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    padding: var(--space-3) var(--space-3) var(--space-1);
  }
  .nav-label:not(:first-child) { margin-top: var(--space-2); }
  .nav-pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    height: 42px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--fs-md);
    font-weight: 500;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--spring);
  }
  .sidebar.collapsed .nav-pill { width: 42px; height: 42px; justify-content: center; padding: 0; }
  .nav-pill :global(svg) { width: 20px; height: 20px; flex-shrink: 0; }
  .nav-pill:hover { background: var(--bg-card-hover); color: var(--text-primary); transform: translateX(2px); }
  .nav-pill:active { transform: translateX(2px) scale(0.99); }
  .nav-pill:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .nav-pill.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 600;
  }
  .nav-pill.active::before {
    content: "";
    position: absolute;
    left: calc(-1 * var(--space-3));
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--accent);
  }
  .sidebar.collapsed .nav-pill.active::before { left: -7px; }
  .nav-pill.active:hover { background: var(--accent-dim); transform: none; }
  .nav-label-text { flex: 1; text-align: left; }
  .nav-counter {
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
  }
  .nav-counter.is-update { background: var(--update-dim); color: var(--update); }
  .nav-pill.active .nav-counter { background: var(--bg-card); color: var(--accent); }
  .nav-counter.is-collapsed {
    position: absolute;
    top: 2px;
    right: 2px;
    padding: 0 4px;
    min-width: 14px;
    font-size: var(--fs-2xs);
    line-height: var(--lh-tight);
  }

  .lang-switcher {
    margin: auto var(--space-4) 0;
    width: auto;
    border-top: 1px solid var(--border);
    border-radius: 0;
    padding: var(--space-4) var(--space-3) var(--space-3);
    height: auto;
  }
  .sidebar.collapsed .lang-switcher {
    width: 42px;
    height: auto;
    margin: auto var(--space-2) 0;
    padding: var(--space-4) 0 0;
    justify-content: center;
    border-radius: 0;
  }
  .lang-switcher:hover { transform: none; }
  .lang-switcher:active { transform: scale(0.99); }
  .lang-switcher.active { background: transparent; color: var(--accent); box-shadow: none; }
  .lang-switcher.active::before { content: none; }
  .lang-switcher.active:hover { background: var(--bg-card-hover); }
  .lang-code { flex-shrink: 0; }

  .sidebar-toggle {
    margin: var(--space-2) var(--space-4) var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    height: 40px;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: var(--letter-tight);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .sidebar-toggle:hover { background: var(--bg-card-hover); color: var(--text-primary); }
  .sidebar-toggle:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .sidebar.collapsed .sidebar-toggle {
    width: 42px;
    height: 42px;
    padding: 0;
    margin: var(--space-2) var(--space-2) var(--space-4);
    justify-content: center;
  }
  .toggle-label { white-space: nowrap; }
</style>
