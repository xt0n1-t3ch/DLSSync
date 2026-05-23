<script lang="ts">
  import { onMount } from "svelte";
  import { searchQuery, currentView } from "../lib/stores";
  import { viewTitle } from "../lib/labels";

  let { onToggleTheme, theme }: { onToggleTheme: () => void; theme: string } = $props();

  let searchInput: HTMLInputElement | undefined = $state();

  onMount(() => {
    const onKeydown = (e: KeyboardEvent): void => {
      if (e.key === "/" && document.activeElement?.tagName !== "INPUT") {
        e.preventDefault();
        searchInput?.focus();
      }
      if (e.key === "Escape" && document.activeElement === searchInput) {
        searchInput?.blur();
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  async function minimize(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }
  async function toggleMaximize(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const maxed = await win.isMaximized();
    if (maxed) {
      await win.unmaximize();
    } else {
      await win.maximize();
    }
  }
  async function close(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }

  let showSearch = $derived($currentView === "library");
  let pageTitle = $derived(showSearch ? "" : viewTitle($currentView));
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="topbar-left" data-tauri-drag-region>
    {#if showSearch}
      <div class="search-wrap">
        <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={searchInput}
          type="search"
          placeholder="Search games"
          bind:value={$searchQuery}
        />
        <span class="kbd">/</span>
      </div>
    {:else if pageTitle}
      <span class="page-title">{pageTitle}</span>
    {/if}
  </div>

  <div class="topbar-right">
    <button class="topbar-btn" title="Toggle theme" onclick={onToggleTheme} aria-label="Toggle theme">
      {#if theme === "dark"}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="4.22" x2="19.78" y2="5.64"/></svg>
      {/if}
    </button>

    <div class="window-controls">
      <button class="win-btn" title="Minimize" onclick={minimize}>
        <svg width="12" height="12" viewBox="0 0 12 12"><line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
      <button class="win-btn" title="Maximize" onclick={toggleMaximize}>
        <svg width="12" height="12" viewBox="0 0 12 12"><rect x="2" y="2" width="8" height="8" stroke="currentColor" stroke-width="1.5" fill="none" rx="1"/></svg>
      </button>
      <button class="win-btn win-close" title="Close" onclick={close}>
        <svg width="12" height="12" viewBox="0 0 12 12"><line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.5"/><line x1="10" y1="2" x2="2" y2="10" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
    </div>
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 20px;
    background: var(--bg-topbar);
    border-bottom: 1px solid var(--border);
    gap: 16px;
    user-select: none;
    -webkit-app-region: drag;
    position: sticky;
    top: 0;
    z-index: 50;
  }
  .topbar-left { flex: 1; min-width: 0; display: flex; align-items: center; }
  .page-title { font-size: 14px; font-weight: 600; color: var(--text-primary); letter-spacing: var(--letter-tight); -webkit-app-region: no-drag; }
  .search-wrap {
    position: relative;
    width: 100%;
    max-width: 320px;
    display: flex;
    align-items: center;
    -webkit-app-region: no-drag;
  }
  .search-icon {
    position: absolute;
    left: 12px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-wrap input {
    width: 100%;
    padding: 8px 38px 8px 34px;
    border-radius: var(--radius-full);
    background: var(--bg-input);
    border: 1px solid var(--border);
    font-size: var(--fs-sm);
    color: var(--text-primary);
  }
  .search-wrap input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-dim); }
  .search-wrap .kbd { position: absolute; right: 10px; pointer-events: none; }

  .topbar-right { display: flex; align-items: center; gap: 6px; -webkit-app-region: no-drag; }
  .topbar-btn { width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; border-radius: var(--radius-md); color: var(--text-muted); transition: color 0.15s var(--ease), background 0.15s var(--ease); }
  .topbar-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }

  .window-controls { display: flex; gap: 2px; margin-left: 6px; padding-left: 6px; border-left: 1px solid var(--border); }
  .win-btn { width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; border-radius: var(--radius-md); color: var(--text-muted); transition: all 0.15s var(--ease); }
  .win-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .win-close:hover { color: #fff; background: var(--danger); }
</style>
