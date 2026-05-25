<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    commandPaletteOpen,
    shortcutOverlayOpen,
    currentView,
    settings,
    persistSettings,
    scanGames,
    loadCatalog,
    triggerThemeToggle,
    triggerApplyAllOutdated,
    triggerUpdateCheck,
    triggerRestoreMostRecent,
  } from "../lib/stores";
  import {
    COMMANDS,
    COMMAND_CATEGORY_LABELS,
    COMMAND_PALETTE_MAX_HEIGHT_PX,
    COMMAND_PALETTE_MAX_WIDTH_PX,
    LIBRARY_VIEW_MODES,
    LIBRARY_DENSITIES,
    matchCommands,
    pushRecentCommand,
    isModifierComboMatch,
    type PaletteCommand,
    type CommandCategory,
  } from "../lib/ux";
  import { getAppPaths, revealPath } from "../lib/api";

  let query = $state("");
  let selectedIndex = $state(0);
  let category = $state<"all" | CommandCategory>("all");
  let inputEl: HTMLInputElement | undefined = $state();

  const categories: readonly ("all" | CommandCategory)[] = ["all", "navigate", "action", "settings"];

  let recentIds = $derived($settings?.ui_prefs.command_palette_recent ?? []);

  let baseCommands = $derived.by(() => {
    if (!query.trim() && recentIds.length > 0) {
      const recentSet = new Set(recentIds);
      const recents = recentIds
        .map((id) => COMMANDS.find((c) => c.id === id))
        .filter((c): c is PaletteCommand => c !== undefined);
      const rest = COMMANDS.filter((c) => !recentSet.has(c.id));
      return [...recents, ...rest];
    }
    return COMMANDS;
  });

  let filtered = $derived.by(() => {
    const pool = category === "all" ? baseCommands : baseCommands.filter((c) => c.category === category);
    const matches = matchCommands(query, pool);
    return matches.map((m) => m.command);
  });

  $effect(() => {
    if ($commandPaletteOpen) {
      query = "";
      selectedIndex = 0;
      void tick().then(() => inputEl?.focus());
    }
  });

  $effect(() => {
    if (selectedIndex >= filtered.length) selectedIndex = Math.max(0, filtered.length - 1);
  });

  onMount(() => {
    const handler = (e: KeyboardEvent): void => {
      const tag = (document.activeElement?.tagName ?? "").toLowerCase();
      const typing = tag === "input" || tag === "textarea" || (document.activeElement as HTMLElement | null)?.isContentEditable === true;
      if (isModifierComboMatch(e, ["mod", "k"])) {
        e.preventDefault();
        commandPaletteOpen.update((v) => !v);
        return;
      }
      if (!typing && e.key === "?") {
        e.preventDefault();
        shortcutOverlayOpen.set(true);
        return;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  function close(): void {
    commandPaletteOpen.set(false);
  }

  function nextCategory(): void {
    const idx = categories.indexOf(category);
    category = categories[(idx + 1) % categories.length];
    selectedIndex = 0;
  }

  async function persistRecent(id: string): Promise<void> {
    if (!$settings) return;
    const next = pushRecentCommand($settings.ui_prefs.command_palette_recent ?? [], id);
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, command_palette_recent: next },
    });
  }

  async function openFolder(kind: "root" | "backups_dir" | "logs_dir"): Promise<void> {
    try {
      const paths = await getAppPaths();
      await revealPath(paths[kind]);
    } catch {
      /* swallow; user already left the palette */
    }
  }

  async function toggleViewMode(): Promise<void> {
    if (!$settings) return;
    const current = $settings.ui_prefs.library_view_mode;
    const next = current === "grid" ? "list" : "grid";
    if (!LIBRARY_VIEW_MODES.includes(next)) return;
    currentView.set("library");
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, library_view_mode: next },
    });
  }

  async function toggleDensity(): Promise<void> {
    if (!$settings) return;
    const current = $settings.ui_prefs.library_density;
    const next = current === "compact" ? "comfy" : "compact";
    if (!LIBRARY_DENSITIES.includes(next)) return;
    currentView.set("library");
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, library_density: next },
    });
  }

  async function setSettingsTab(tab: string): Promise<void> {
    if (!$settings) return;
    currentView.set("settings");
    await persistSettings({
      ...$settings,
      ui_prefs: { ...$settings.ui_prefs, settings_active_tab: tab as never },
    });
  }

  async function runCommand(cmd: PaletteCommand): Promise<void> {
    close();
    await persistRecent(cmd.id);
    switch (cmd.id) {
      case "nav.library": currentView.set("library"); break;
      case "nav.catalog": currentView.set("catalog"); break;
      case "nav.backups": currentView.set("backups"); break;
      case "nav.settings": currentView.set("settings"); break;
      case "nav.about": currentView.set("about"); break;
      case "action.apply_all_outdated":
        currentView.set("library");
        triggerApplyAllOutdated();
        break;
      case "action.rescan": void scanGames(); break;
      case "action.refresh_manifest": void loadCatalog(); break;
      case "action.check_updates": triggerUpdateCheck(); break;
      case "action.restore_recent":
        currentView.set("backups");
        triggerRestoreMostRecent();
        break;
      case "action.open_data_folder": void openFolder("root"); break;
      case "action.open_backups_folder": void openFolder("backups_dir"); break;
      case "action.open_logs_folder": void openFolder("logs_dir"); break;
      case "action.toggle_theme": triggerThemeToggle(); break;
      case "action.toggle_view_mode": void toggleViewMode(); break;
      case "action.toggle_density": void toggleDensity(); break;
      case "settings.general": void setSettingsTab("general"); break;
      case "settings.updates": void setSettingsTab("updates"); break;
      case "settings.detection": void setSettingsTab("detection"); break;
      case "settings.art": void setSettingsTab("art"); break;
      case "settings.advanced": void setSettingsTab("advanced"); break;
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      nextCategory();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (filtered.length === 0) return;
      selectedIndex = (selectedIndex + 1) % filtered.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (filtered.length === 0) return;
      selectedIndex = (selectedIndex - 1 + filtered.length) % filtered.length;
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const cmd = filtered[selectedIndex];
      if (cmd) void runCommand(cmd);
    }
  }

  function onBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) close();
  }
</script>

{#if $commandPaletteOpen}
  <div
    class="palette-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Command palette"
    onclick={onBackdropClick}
    onkeydown={(e) => { if (e.key === "Escape") close(); }}
    tabindex="-1"
  >
    <div class="palette" style="max-width: {COMMAND_PALETTE_MAX_WIDTH_PX}px; max-height: {COMMAND_PALETTE_MAX_HEIGHT_PX}px;">
      <div class="palette-search">
        <svg class="palette-search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKeydown}
          type="text"
          placeholder="Search commands or jump to a view"
          spellcheck="false"
          autocomplete="off"
        />
      </div>

      <div class="palette-categories">
        {#each categories as cat}
          <button
            class="category"
            class:active={category === cat}
            onclick={() => { category = cat; selectedIndex = 0; inputEl?.focus(); }}
          >{COMMAND_CATEGORY_LABELS[cat]}</button>
        {/each}
        <span class="tab-hint">Tab to cycle</span>
      </div>

      <div class="palette-results">
        {#if filtered.length === 0}
          <div class="empty">No matching commands</div>
        {:else}
          {#each filtered as cmd, i (cmd.id)}
            <button
              class="result"
              class:active={i === selectedIndex}
              onclick={() => void runCommand(cmd)}
              onmouseenter={() => { selectedIndex = i; }}
            >
              <span class="result-tag" data-cat={cmd.category}>{COMMAND_CATEGORY_LABELS[cmd.category]}</span>
              <span class="title">{cmd.title}</span>
              {#if !query.trim() && recentIds.includes(cmd.id)}
                <span class="recent-tag">recent</span>
              {/if}
              {#if i === selectedIndex}
                <span class="enter-hint" aria-hidden="true">↵</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <div class="palette-footer">
        <span class="footer-hint"><span class="kbd">↑</span><span class="kbd">↓</span> navigate</span>
        <span class="footer-hint"><span class="kbd">↵</span> run</span>
        <span class="footer-hint"><span class="kbd">Tab</span> category</span>
        <span class="footer-hint"><span class="kbd">Esc</span> close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    z-index: 200;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    animation: fadeIn var(--dur-fast) var(--ease-out);
  }
  .palette {
    width: 92%;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: pop-in var(--dur-fast) var(--ease-out);
  }
  .palette-search {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 20px 22px;
    color: var(--text-muted);
  }
  .palette-search-icon { flex-shrink: 0; }
  .palette-search input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 19px;
    font-weight: 500;
    letter-spacing: var(--letter-tight);
    padding: 0;
    outline: none;
  }
  .palette-search input::placeholder { color: var(--text-placeholder); font-weight: 400; }
  .palette-categories {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 18px 12px;
  }
  .category {
    font-size: var(--fs-xs);
    font-weight: 600;
    padding: 5px 12px;
    border-radius: var(--radius-full);
    color: var(--text-muted);
    background: transparent;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .category:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .category.active { color: var(--accent-fg); background: var(--accent); }
  .tab-hint { margin-left: auto; font-size: var(--fs-2xs); opacity: 0.7; }
  .palette-results {
    flex: 1;
    overflow-y: auto;
    padding: 6px 10px;
    border-top: 1px solid var(--border);
  }
  .empty {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
  .result {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 11px 14px;
    border-radius: var(--radius-lg);
    text-align: left;
    color: var(--text-secondary);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .result.active { background: var(--accent-soft); color: var(--text-primary); }
  .result-tag {
    flex-shrink: 0;
    width: 64px;
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .result-tag[data-cat="navigate"] { color: var(--badge-blue-fg); }
  .result-tag[data-cat="action"] { color: var(--badge-green-fg); }
  .result-tag[data-cat="settings"] { color: var(--badge-purple-fg); }
  .title { flex: 1; font-size: var(--fs-sm); font-weight: 500; }
  .recent-tag {
    font-size: 9px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 700;
  }
  .enter-hint {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--accent);
    font-size: var(--fs-xs);
  }
  .palette-footer {
    display: flex;
    gap: 14px;
    padding: 10px 18px;
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: var(--fs-2xs);
    color: var(--text-muted);
  }
  .footer-hint { display: inline-flex; align-items: center; gap: 4px; }
</style>
