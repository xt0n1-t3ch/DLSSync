<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Toast from "./components/Toast.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import EfficiencyModeController from "./components/EfficiencyModeController.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import NotificationsBell from "./components/NotificationsBell.svelte";
  import LanguageMenu from "./components/LanguageMenu.svelte";
  import ShortcutOverlay from "./components/ShortcutOverlay.svelte";
  import SupportNudge from "./components/SupportNudge.svelte";
  import ApplyProgressModal from "./components/ApplyProgressModal.svelte";
  import ActivityDock from "./components/ActivityDock.svelte";
  import Library from "./views/Library.svelte";
  import GameDetailDrawer from "./components/GameDetailDrawer.svelte";
  import Catalog from "./views/Catalog.svelte";
  import Backups from "./views/Backups.svelte";
  import Drivers from "./views/Drivers.svelte";
  import Settings from "./views/Settings.svelte";
  import About from "./views/About.svelte";
  import {
    currentView,
    drawerGameId,
    loadSettings,
    settings,
    persistSettings,
    bootstrapCatalog,
    requestThemeToggle,
    applyModalOpen,
    notificationsOpen,
    languageMenuOpen,
  } from "./lib/stores";
  import { activeArt, clearActiveArt } from "./lib/artContext";
  import { coverAccent } from "./lib/coverAccent";
  import { installApplyEventListeners } from "./lib/applyEvents";
  import { installBackgroundScanListeners } from "./lib/backgroundScan";
  import {
    installDriverInstallListener,
    installSystemDriverListener,
  } from "./lib/driverInstallEvents";
  import { isLocale, localeFromNavigator, setLocale, t } from "./lib/i18n/index";
  import { motionDuration } from "./lib/ux";

  let theme = $state(localStorage.getItem("dlssync-theme") || "dark");

  function toggleTheme(): void {
    theme = theme === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("dlssync-theme", theme);
    if ($settings) {
      void persistSettings({
        ...$settings,
        ui_prefs: { ...$settings.ui_prefs, theme },
      });
    }
  }

  let collapsed = $derived($settings?.ui_prefs.sidebar_collapsed ?? false);
  let railGameId = $derived($currentView === "library" ? $drawerGameId : null);

  let gameAccent = $state<string | null>(null);
  $effect(() => {
    const url = $activeArt;
    if (!url) {
      gameAccent = null;
      return;
    }
    let stale = false;
    void coverAccent(url).then((color) => {
      if (!stale) gameAccent = color;
    });
    return () => {
      stale = true;
    };
  });

  onMount(async () => {
    await loadSettings();
    if ($settings) {
      const persistedTheme = $settings.ui_prefs.theme;
      if (persistedTheme && persistedTheme !== theme) {
        theme = persistedTheme;
      }
      const persistedLocale = $settings.ui_prefs.language;
      if (isLocale(persistedLocale)) {
        setLocale(persistedLocale);
      } else {
        const guess = localeFromNavigator();
        setLocale(guess);
        void persistSettings({
          ...$settings,
          ui_prefs: { ...$settings.ui_prefs, language: guess },
        });
      }
    }
    document.documentElement.setAttribute("data-theme", theme);
    void installApplyEventListeners();
    void installBackgroundScanListeners();
    void installDriverInstallListener();
    void installSystemDriverListener();
    void bootstrapCatalog();
  });

  let lastThemeSignal = $state(0);
  $effect(() => {
    const n = $requestThemeToggle;
    if (n !== lastThemeSignal) {
      lastThemeSignal = n;
      if (n > 0) toggleTheme();
    }
  });

  $effect(() => {
    if ($currentView !== "library") {
      clearActiveArt();
      if ($drawerGameId) drawerGameId.set(null);
    }
  });

  onMount(() => {
    const onKey = async (e: KeyboardEvent): Promise<void> => {
      if (e.key === "F12") {
        e.preventDefault();
        try {
          await invoke("open_devtools");
        } catch {
          /* outside tauri context */
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<div class="app-shell" class:rail-open={!!railGameId} class:sidebar-collapsed={collapsed}>
  <div class="app-ambient" aria-hidden="true" style:--game-accent={gameAccent}>
    <div class="ambient-mesh"></div>
    {#if $activeArt}
      <img class="ambient-art" src={$activeArt} alt="" transition:fade={{ duration: motionDuration(600) }} />
    {/if}
    {#if gameAccent}
      <div class="ambient-accent"></div>
    {/if}
    <div class="ambient-grain"></div>
  </div>
  <Sidebar />
  <TopBar onToggleTheme={toggleTheme} {theme} />
  <div class="app-main">
    <main class="main-content">
      <div class="main-inner">
        <div class="main-primary">
          {#if $currentView === "library"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-library"><Library /></div>
          {:else if $currentView === "catalog"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-catalog"><Catalog /></div>
          {:else if $currentView === "backups"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-backups"><Backups /></div>
          {:else if $currentView === "drivers"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-drivers"><Drivers /></div>
          {:else if $currentView === "settings"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-settings">
              <Settings onToggleTheme={toggleTheme} currentTheme={theme} />
            </div>
          {:else if $currentView === "about"}
            <div in:fly={{ y: 8, duration: motionDuration(200) }} data-testid="view-about"><About /></div>
          {/if}
        </div>
      </div>
    </main>
  </div>
  {#if railGameId}
    <button class="rail-scrim" aria-label={$t("common.close")} onclick={() => drawerGameId.set(null)}></button>
    <aside class="detail-rail" class:has-rail={!!railGameId} in:fly={{ x: 24, duration: motionDuration(220) }}>
      <GameDetailDrawer
        gameId={railGameId}
        onClose={() => drawerGameId.set(null)}
        onApplyStart={() => applyModalOpen.set(true)}
      />
    </aside>
  {/if}
</div>
<Toast />
<ActivityDock />
<EfficiencyModeController />
<UpdateBanner />
<CommandPalette />
<NotificationsBell open={$notificationsOpen} onClose={() => notificationsOpen.set(false)} />
<LanguageMenu open={$languageMenuOpen} onClose={() => languageMenuOpen.set(false)} />
<ShortcutOverlay />
<SupportNudge />
{#if $applyModalOpen}
  <ApplyProgressModal onClose={() => applyModalOpen.set(false)} />
{/if}

<style>
  /* One cohesive floating app surface (sidebar + topbar + content integrated),
     on the subtle ambient backdrop. Rounded + overflow-clipped = the "docker" feel. */
  /* Edge-to-edge: the app fills the window (no inner margin → no square frame).
     The window's own corners are rounded by the OS. */
  .app-shell {
    position: fixed;
    inset: 0;
    z-index: 1;
    display: grid;
    grid-template-rows: var(--topbar-height) 1fr;
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr) 0fr;
    overflow: hidden;
    background: transparent;
    transition: grid-template-columns var(--dur-normal) var(--ease-out);
  }
  .app-shell.sidebar-collapsed {
    grid-template-columns: var(--sidebar-width-collapsed) minmax(0, 1fr) 0fr;
  }
  .app-shell.rail-open {
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr) var(--drawer-width);
  }
  .app-shell.rail-open.sidebar-collapsed {
    grid-template-columns: var(--sidebar-width-collapsed) minmax(0, 1fr) var(--drawer-width);
  }
  .app-shell :global(.sidebar) {
    grid-row: 1 / -1;
    grid-column: 1;
    height: 100%;
    z-index: 70;
  }
  .app-shell :global(.topbar) {
    grid-row: 1;
    grid-column: 2 / -1;
    position: relative;
    z-index: 70;
  }
  @media (prefers-reduced-motion: reduce) {
    .app-shell { transition: none; }
  }
  .app-ambient {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    background: var(--bg-base);
  }
  .ambient-mesh {
    position: absolute;
    inset: -20%;
    background:
      radial-gradient(ellipse 72% 62% at 5% 0%, var(--ambient-glow-1), transparent 60%),
      radial-gradient(ellipse 60% 55% at 98% 6%, var(--ambient-glow-2), transparent 62%),
      radial-gradient(ellipse 82% 78% at 0% 100%, var(--ambient-glow-3), transparent 60%);
    animation: ambient-drift 64s ease-in-out infinite alternate;
  }
  /* Tidal-style art tint: a heavily blurred + darkened copy of the focused game's
     cover, behind all chrome so the frosted glass refracts real colour. */
  .ambient-art {
    position: absolute;
    inset: -15%;
    width: 130%;
    height: 130%;
    object-fit: cover;
    filter: blur(var(--art-blur)) saturate(var(--art-sat)) brightness(var(--art-dim));
    opacity: var(--art-opacity);
    pointer-events: none;
    -webkit-mask-image:
      linear-gradient(to right, #000 var(--art-edge-left), transparent var(--art-edge-left-fade)),
      linear-gradient(to bottom, #000 var(--art-edge-top), transparent var(--art-edge-top-fade));
    mask-image:
      linear-gradient(to right, #000 var(--art-edge-left), transparent var(--art-edge-left-fade)),
      linear-gradient(to bottom, #000 var(--art-edge-top), transparent var(--art-edge-top-fade));
    mask-composite: add;
  }
  @keyframes ambient-drift {
    from { transform: translate3d(0, 0, 0) scale(1); }
    to { transform: translate3d(2.5%, -2%, 0) scale(1.1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ambient-mesh { animation: none; }
  }
  .ambient-grain {
    position: absolute;
    inset: 0;
    background-image: var(--noise-url);
    opacity: 0.4;
    mix-blend-mode: overlay;
  }
  /* The content column: the topbar overlays the top so scrolling content frosts
     under it (real glass), and the sidebar is an integrated column to the left. */
  .app-main {
    grid-row: 2;
    grid-column: 2;
    position: relative;
    z-index: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
    background: transparent;
  }
  .main-inner {
    max-width: var(--content-max);
    padding: clamp(18px, 2.4vw, 36px) clamp(18px, 3.2vw, 48px) clamp(16px, 2vw, 28px);
    margin: 0 auto;
  }
  .main-primary { min-width: 0; }

  .detail-rail {
    grid-row: 2;
    grid-column: 3;
    position: relative;
    z-index: 2;
    height: 100%;
    width: var(--drawer-width);
    overflow: hidden;
    border-inline-start: 1px solid var(--border);
  }
  .detail-rail :global(.detail-view) { width: 100%; height: 100%; }
  .rail-scrim { display: none; }

  @media (min-width: 1120px) {
    .app-shell.rail-open {
      grid-template-columns: var(--sidebar-width) minmax(var(--rail-width), 1fr) var(--drawer-width);
    }
    .app-shell.rail-open.sidebar-collapsed {
      grid-template-columns: var(--sidebar-width-collapsed) minmax(var(--rail-width), 1fr) var(--drawer-width);
    }
  }

  @media (max-width: 1119px) {
    .app-shell.rail-open,
    .app-shell.rail-open.sidebar-collapsed {
      grid-template-columns: var(--sidebar-width) minmax(0, 1fr) 0fr;
    }
    .app-shell.rail-open.sidebar-collapsed {
      grid-template-columns: var(--sidebar-width-collapsed) minmax(0, 1fr) 0fr;
    }
    .rail-scrim {
      display: block;
      position: fixed;
      inset: var(--topbar-height) 0 0 0;
      z-index: 59;
      background: var(--bg-overlay);
      border: none;
      cursor: pointer;
    }
    .detail-rail {
      position: fixed;
      top: var(--topbar-height);
      right: 0;
      bottom: 0;
      width: min(95vw, var(--drawer-width));
      height: auto;
      z-index: 60;
      border-inline-start: 1px solid var(--border);
    }
  }

  @media (max-width: 720px) {
    .main-inner { padding: 16px 16px 20px; }
  }
</style>
