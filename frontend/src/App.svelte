<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Toast from "./components/Toast.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import NotificationsBell from "./components/NotificationsBell.svelte";
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
  } from "./lib/stores";
  import { activeArt, clearActiveArt } from "./lib/artContext";
  import { installApplyEventListeners } from "./lib/applyEvents";
  import {
    installDriverInstallListener,
    installSystemDriverListener,
  } from "./lib/driverInstallEvents";

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

  onMount(async () => {
    await loadSettings();
    if ($settings) {
      const persistedTheme = $settings.ui_prefs.theme;
      if (persistedTheme && persistedTheme !== theme) {
        theme = persistedTheme;
      }
    }
    document.documentElement.setAttribute("data-theme", theme);
    void installApplyEventListeners();
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
          const { invoke } = await import("@tauri-apps/api/core");
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

<div class="app-shell">
  <div class="app-ambient" aria-hidden="true">
    <div class="ambient-mesh"></div>
    {#if $activeArt}
      <img class="ambient-art" src={$activeArt} alt="" transition:fade={{ duration: 600 }} />
    {/if}
    <div class="ambient-grain"></div>
  </div>
  <Sidebar />
  <div class="app-main" class:sidebar-collapsed={collapsed}>
    <TopBar onToggleTheme={toggleTheme} {theme} />
    <main class="main-content">
      <div class="main-inner">
        {#if $currentView === "library" && $drawerGameId}
          <div in:fly={{ y: 8, duration: 200 }}>
            <GameDetailDrawer
              gameId={$drawerGameId}
              onClose={() => drawerGameId.set(null)}
              onApplyStart={() => applyModalOpen.set(true)}
            />
          </div>
        {:else if $currentView === "library"}
          <div in:fly={{ y: 8, duration: 200 }}><Library /></div>
        {:else if $currentView === "catalog"}
          <div in:fly={{ y: 8, duration: 200 }}><Catalog /></div>
        {:else if $currentView === "backups"}
          <div in:fly={{ y: 8, duration: 200 }}><Backups /></div>
        {:else if $currentView === "drivers"}
          <div in:fly={{ y: 8, duration: 200 }}><Drivers /></div>
        {:else if $currentView === "settings"}
          <div in:fly={{ y: 8, duration: 200 }}>
            <Settings onToggleTheme={toggleTheme} currentTheme={theme} />
          </div>
        {:else if $currentView === "about"}
          <div in:fly={{ y: 8, duration: 200 }}><About /></div>
        {/if}
      </div>
    </main>
  </div>
</div>
<Toast />
<ActivityDock />
<UpdateBanner />
<CommandPalette />
<NotificationsBell open={$notificationsOpen} onClose={() => notificationsOpen.set(false)} />
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
    display: flex;
    overflow: hidden;
    background: transparent;
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
    position: relative;
    z-index: 1;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    background: transparent;
    padding-top: var(--topbar-height);
  }
  .main-inner {
    max-width: var(--content-max);
    padding: clamp(18px, 2.4vw, 36px) clamp(18px, 3.2vw, 48px) clamp(16px, 2vw, 28px);
    margin: 0 auto;
  }
  @media (max-width: 720px) {
    .main-inner { padding: 16px 16px 20px; }
  }
</style>
