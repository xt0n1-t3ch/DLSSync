<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import Sidebar from "./components/Sidebar.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Toast from "./components/Toast.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import Library from "./views/Library.svelte";
  import Catalog from "./views/Catalog.svelte";
  import Backups from "./views/Backups.svelte";
  import Settings from "./views/Settings.svelte";
  import About from "./views/About.svelte";
  import { currentView, drawerGameId, loadSettings, settings, persistSettings, bootstrapCatalog } from "./lib/stores";

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
    void bootstrapCatalog();
  });
</script>

<Sidebar />
<div class="main-wrapper" class:sidebar-collapsed={collapsed} data-drawer-open={$drawerGameId ? "true" : "false"}>
  <TopBar onToggleTheme={toggleTheme} {theme} />
  <main class="main-content">
    <div class="main-inner">
      {#if $currentView === "library"}
        <div in:fly={{ y: 8, duration: 200 }}><Library /></div>
      {:else if $currentView === "catalog"}
        <div in:fly={{ y: 8, duration: 200 }}><Catalog /></div>
      {:else if $currentView === "backups"}
        <div in:fly={{ y: 8, duration: 200 }}><Backups /></div>
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
<Toast />
<UpdateBanner />

<style>
  .main-wrapper {
    margin-left: var(--sidebar-width);
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    transition: margin-left 0.22s var(--ease), padding-right 0.24s var(--ease);
  }
  .main-wrapper.sidebar-collapsed {
    margin-left: var(--sidebar-width-collapsed);
  }
  .main-wrapper[data-drawer-open="true"] {
    padding-right: var(--drawer-width);
  }
  @media (max-width: 1100px) {
    .main-wrapper, .main-wrapper.sidebar-collapsed { margin-left: var(--sidebar-width-collapsed); }
  }
  @media (max-width: 960px) {
    .main-wrapper[data-drawer-open="true"] { padding-right: 0; }
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    max-height: calc(100vh - var(--topbar-height));
    background: var(--bg-app);
  }
  .main-inner {
    max-width: var(--content-max);
    padding: 28px 32px 24px;
    margin: 0 auto;
  }
</style>
