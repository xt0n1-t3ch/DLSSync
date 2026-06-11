<script lang="ts">
  import { onMount } from "svelte";
  import {
    searchQuery,
    currentView,
    commandPaletteOpen,
    notificationsOpen,
    notificationsUnreadCount,
  } from "../lib/stores";
  import { t } from "../lib/i18n/index";

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

  function openPalette(): void {
    commandPaletteOpen.set(true);
  }

  let showSearch = $derived($currentView === "library");
  let unread = $derived($notificationsUnreadCount);
  let modKeyLabel = $derived(typeof navigator !== "undefined" && navigator.platform.toLowerCase().includes("mac") ? "⌘" : "Ctrl");
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
          placeholder={$t("component.chrome.topbar.searchPlaceholder")}
          bind:value={$searchQuery}
        />
        <span class="kbd">/</span>
      </div>
    {/if}
  </div>

  <div class="topbar-right">
    <button class="topbar-btn palette-btn" title={$t("component.chrome.topbar.commandPaletteTitle", { mod: modKeyLabel })} onclick={openPalette} aria-label={$t("component.chrome.topbar.commandPaletteAria")}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
    </button>

    <div class="bell-wrap">
      <button
        class="topbar-btn bell-btn"
        title={$t("component.chrome.topbar.notifications")}
        data-notifications-toggle
        onclick={() => notificationsOpen.update((v) => !v)}
        aria-haspopup="dialog"
        aria-expanded={$notificationsOpen}
        aria-label={unread > 0 ? $t("component.chrome.topbar.unreadNotifications", { count: unread }) : $t("component.chrome.topbar.notifications")}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/>
          <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>
        </svg>
        {#if unread > 0}
          <span class="bell-badge" aria-hidden="true">{unread > 9 ? "9+" : unread}</span>
        {/if}
      </button>
    </div>

    <button class="topbar-btn" title={$t("component.chrome.topbar.toggleTheme")} onclick={onToggleTheme} aria-label={$t("component.chrome.topbar.toggleTheme")}>
      {#if theme === "dark"}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="4.22" x2="19.78" y2="5.64"/></svg>
      {/if}
    </button>

    <div class="window-controls">
      <button class="win-btn win-quiet" title={$t("component.chrome.topbar.minimize")} onclick={minimize} aria-label={$t("component.chrome.topbar.minimizeAria")}>
        <svg width="12" height="12" viewBox="0 0 12 12"><line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
      <button class="win-btn win-quiet" title={$t("component.chrome.topbar.maximize")} onclick={toggleMaximize} aria-label={$t("component.chrome.topbar.maximizeAria")}>
        <svg width="12" height="12" viewBox="0 0 12 12"><rect x="2" y="2" width="8" height="8" stroke="currentColor" stroke-width="1.5" fill="none" rx="1"/></svg>
      </button>
      <button class="win-btn win-close" title={$t("common.close")} onclick={close} aria-label={$t("component.chrome.topbar.closeAria")}>
        <svg width="12" height="12" viewBox="0 0 12 12"><line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.5"/><line x1="10" y1="2" x2="2" y2="10" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
    </div>
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 20px;
    background: var(--glass-2);
    backdrop-filter: var(--glass-blur-bar);
    -webkit-backdrop-filter: var(--glass-blur-bar);
    border-bottom: 1px solid var(--border);
    box-shadow: var(--glass-edge);
    gap: 16px;
    user-select: none;
  }
  @supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .topbar { background: var(--glass-fallback); }
  }
  .topbar-left { flex: 1; min-width: 0; display: flex; align-items: center; }
  .search-wrap {
    position: relative;
    width: 100%;
    max-width: 320px;
    display: flex;
    align-items: center;
  }
  .search-icon {
    position: absolute;
    left: 14px;
    color: var(--text-muted);
    pointer-events: none;
  }
  .search-wrap input {
    width: 100%;
    height: 36px;
    padding: 0 38px 0 36px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    border: 1px solid transparent;
    font-size: var(--fs-sm);
    color: var(--text-primary);
  }
  .search-wrap input:hover { background: var(--bg-card-hover); }
  .search-wrap input:focus { background: var(--bg-card); border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-dim); }
  .search-wrap .kbd { position: absolute; right: 12px; pointer-events: none; }

  .topbar-right { display: flex; align-items: center; gap: 6px; }

  .bell-wrap { position: relative; display: inline-flex; }
  .topbar-btn { width: 30px; height: 30px; display: flex; align-items: center; justify-content: center; border-radius: var(--radius-md); color: var(--text-muted); transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease); position: relative; }
  .topbar-btn:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .topbar-btn:focus-visible { color: var(--text-primary); outline: none; box-shadow: var(--shadow-ring); }

  .bell-badge {
    position: absolute;
    top: 1px;
    right: 1px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: var(--radius-full);
    background: var(--update);
    color: var(--update-fg);
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }

  .window-controls { display: flex; gap: 2px; margin-left: 8px; padding-left: 8px; border-left: 1px solid var(--border); }
  .win-btn {
    width: 34px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .win-btn:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .win-quiet { color: var(--text-placeholder); }
  .win-quiet:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .win-close { color: var(--text-muted); }
  .win-close:hover { color: #fff; background: var(--danger); }
</style>
