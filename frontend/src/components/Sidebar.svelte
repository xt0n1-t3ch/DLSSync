<script lang="ts">
  import { onMount } from "svelte";
  import { currentView, settings, persistSettings } from "../lib/stores";

  let appVersion = $state("dev");
  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      appVersion = await getVersion();
    } catch {
      appVersion = "dev";
    }
  });

  type NavItem = { id: string; icon: string; title: string };

  const librarySection: NavItem[] = [
    { id: "library", icon: "library", title: "Library" },
    { id: "catalog", icon: "catalog", title: "Catalog" },
    { id: "backups", icon: "backups", title: "Backups" },
  ];

  const settingsSection: NavItem[] = [
    { id: "settings", icon: "settings", title: "Settings" },
    { id: "about", icon: "about", title: "About" },
  ];

  function switchView(id: string): void {
    currentView.set(id);
  }

  let collapsed = $derived($settings?.ui_prefs.sidebar_collapsed ?? false);

  async function toggleCollapsed(): Promise<void> {
    if (!$settings) return;
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, sidebar_collapsed: !collapsed },
    });
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-brand">
    <button class="brand-mark" title={collapsed ? "Expand sidebar" : "Collapse sidebar"} onclick={toggleCollapsed}>
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 12a9 9 0 1 0 3-6.7"/>
        <polyline points="3 4 3 9 8 9"/>
      </svg>
    </button>
    {#if !collapsed}
      <span class="brand-wrap">
        <span class="brand-name">DLSSync</span>
        <span class="brand-version mono">v{appVersion}</span>
      </span>
    {/if}
  </div>

  <nav class="sidebar-nav">
    {#if !collapsed}<div class="nav-label">Library</div>{/if}
    {#each librarySection as item}
      <button class="nav-btn" class:active={$currentView === item.id} title={item.title} onclick={() => switchView(item.id)}>
        {#if item.icon === "library"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/></svg>
        {:else if item.icon === "catalog"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>
        {:else if item.icon === "backups"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5" rx="0.5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>
        {/if}
        {#if !collapsed}<span class="nav-label-text">{item.title}</span>{/if}
        {#if item.id === "backups" && $currentView !== "backups"}
          <span class="nav-pill"></span>
        {/if}
      </button>
    {/each}

    {#if !collapsed}<div class="nav-label">General</div>{/if}
    {#each settingsSection as item}
      <button class="nav-btn" class:active={$currentView === item.id} title={item.title} onclick={() => switchView(item.id)}>
        {#if item.icon === "settings"}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
        {/if}
        {#if !collapsed}<span class="nav-label-text">{item.title}</span>{/if}
      </button>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--sidebar-width);
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    z-index: 100;
    transition: width 0.22s var(--ease);
  }
  .sidebar.collapsed {
    width: var(--sidebar-width-collapsed);
  }
  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    height: var(--topbar-height);
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }
  .sidebar.collapsed .sidebar-brand { padding: 0; justify-content: center; }
  .brand-mark {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s var(--ease), color 0.15s var(--ease);
  }
  .brand-mark:hover { background: var(--accent-glow); }
  .brand-wrap { display: flex; flex-direction: column; gap: 0; line-height: 1.1; min-width: 0; }
  .brand-name {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }
  .brand-version { font-size: 9.5px; color: var(--text-muted); margin-top: 2px; letter-spacing: 0; font-variant-numeric: tabular-nums; }
  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 12px;
    flex: 1;
    overflow-y: auto;
  }
  .sidebar.collapsed .sidebar-nav { padding: 12px 8px; align-items: center; }
  .nav-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    padding: 14px 10px 6px;
  }
  .nav-btn {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 9px 10px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    transition: background 0.15s var(--ease), color 0.15s var(--ease);
  }
  .sidebar.collapsed .nav-btn { width: 40px; height: 40px; justify-content: center; padding: 0; }
  .nav-btn :global(svg) { width: 18px; height: 18px; flex-shrink: 0; }
  .nav-btn:hover { background: var(--bg-card); color: var(--text-primary); }
  .nav-btn.active {
    background: var(--accent-dim);
    color: var(--accent);
  }
  .nav-btn.active::before {
    content: '';
    position: absolute;
    left: -12px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 60%;
    background: var(--accent);
    border-radius: 0 var(--radius-xs) var(--radius-xs) 0;
  }
  .sidebar.collapsed .nav-btn.active::before { display: none; }
  .nav-label-text { flex: 1; }
  .nav-pill { display: none; }
  @media (max-width: 1100px) {
    .sidebar { width: var(--sidebar-width-collapsed); }
    .sidebar-brand { padding: 0; justify-content: center; }
    .brand-wrap { display: none; }
    .sidebar-nav { padding: 12px 8px; align-items: center; }
    .nav-label { display: none; }
    .nav-btn { width: 40px; height: 40px; justify-content: center; padding: 0; }
    .nav-btn.active::before { display: none; }
    .nav-label-text { display: none; }
  }
</style>
