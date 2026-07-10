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
    LIBRARY_VIEW_MODES,
    LIBRARY_DENSITIES,
    matchCommands,
    matchedIndices,
    highlightSegments,
    pushRecentCommand,
    isModifierComboMatch,
    type PaletteCommand,
    type CommandCategory,
  } from "../lib/ux";
  import { getAppPaths, revealPath } from "../lib/api";
  import { focusTrap } from "../actions/focusTrap";
  import { t } from "../lib/i18n/index";
  import Search from "@lucide/svelte/icons/search";
  import Command from "@lucide/svelte/icons/command";
  import LayoutGrid from "@lucide/svelte/icons/layout-grid";
  import LayoutList from "@lucide/svelte/icons/layout-list";
  import Layers from "@lucide/svelte/icons/layers";
  import Archive from "@lucide/svelte/icons/archive";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import DownloadCloud from "@lucide/svelte/icons/download-cloud";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import RotateCw from "@lucide/svelte/icons/rotate-cw";
  import ArrowUpCircle from "@lucide/svelte/icons/arrow-up-circle";
  import Undo2 from "@lucide/svelte/icons/undo-2";
  import Folder from "@lucide/svelte/icons/folder";
  import FolderArchive from "@lucide/svelte/icons/folder-archive";
  import ScrollText from "@lucide/svelte/icons/scroll-text";
  import SunMoon from "@lucide/svelte/icons/sun-moon";
  import Rows3 from "@lucide/svelte/icons/rows-3";
  import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
  import Radar from "@lucide/svelte/icons/radar";
  import Image from "@lucide/svelte/icons/image";
  import Wrench from "@lucide/svelte/icons/wrench";

  const ICONS: Record<string, typeof LayoutGrid> = {
    "layout-grid": LayoutGrid,
    "layout-list": LayoutList,
    layers: Layers,
    archive: Archive,
    settings: Settings,
    info: Info,
    "download-cloud": DownloadCloud,
    "refresh-cw": RefreshCw,
    "rotate-cw": RotateCw,
    "arrow-up-circle": ArrowUpCircle,
    "undo-2": Undo2,
    folder: Folder,
    "folder-archive": FolderArchive,
    "scroll-text": ScrollText,
    "sun-moon": SunMoon,
    "rows-3": Rows3,
    "sliders-horizontal": SlidersHorizontal,
    radar: Radar,
    image: Image,
    wrench: Wrench,
  };

  let query = $state("");
  let selectedIndex = $state(0);
  let category = $state<"all" | CommandCategory>("all");
  let inputEl: HTMLInputElement | undefined = $state();
  let resultsEl: HTMLDivElement | undefined = $state();

  const categories: readonly ("all" | CommandCategory)[] = ["all", "navigate", "action", "settings"];
  const RESULT_CATEGORIES: readonly CommandCategory[] = ["navigate", "action", "settings"];

  let recentIds = $derived($settings?.ui_prefs.command_palette_recent ?? []);

  function cmdTitle(cmd: PaletteCommand): string {
    return $t("command." + cmd.id + ".title");
  }
  function cmdHint(cmd: PaletteCommand): string | undefined {
    if (cmd.hint === undefined) return undefined;
    return $t("command." + cmd.id + ".hint");
  }
  function categoryLabel(cat: "all" | CommandCategory): string {
    return $t("commandCategory." + cat);
  }

  interface ResultItem {
    cmd: PaletteCommand;
    ranges: number[];
    index: number;
  }
  interface ResultGroup {
    key: string;
    label: string;
    items: ResultItem[];
  }

  let view = $derived.by<{ groups: ResultGroup[]; flat: PaletteCommand[] }>(() => {
    const q = query.trim();
    const raw: { key: string; label: string; cmds: { cmd: PaletteCommand; ranges: number[] }[] }[] = [];

    if (!q) {
      if (category === "all" && recentIds.length > 0) {
        const recents = recentIds
          .map((id) => COMMANDS.find((c) => c.id === id))
          .filter((c): c is PaletteCommand => c !== undefined);
        if (recents.length > 0) {
          raw.push({ key: "recent", label: $t("component.palette.recent"), cmds: recents.map((cmd) => ({ cmd, ranges: [] })) });
        }
      }
      const exclude = new Set(category === "all" ? recentIds : []);
      for (const cat of RESULT_CATEGORIES) {
        if (category !== "all" && category !== cat) continue;
        const cmds = COMMANDS.filter((c) => c.category === cat && !exclude.has(c.id)).map((cmd) => ({ cmd, ranges: [] as number[] }));
        if (cmds.length > 0) raw.push({ key: cat, label: categoryLabel(cat), cmds });
      }
    } else {
      const pool = category === "all" ? COMMANDS : COMMANDS.filter((c) => c.category === category);
      const matches = matchCommands(q, pool);
      for (const cat of RESULT_CATEGORIES) {
        if (category !== "all" && category !== cat) continue;
        const cmds = matches
          .filter((m) => m.command.category === cat)
          .map((m) => ({ cmd: m.command, ranges: matchedIndices(q, cmdTitle(m.command)) }));
        if (cmds.length > 0) raw.push({ key: cat, label: categoryLabel(cat), cmds });
      }
    }

    const flat: PaletteCommand[] = [];
    const groups: ResultGroup[] = raw.map((g) => ({
      key: g.key,
      label: g.label,
      items: g.cmds.map(({ cmd, ranges }) => {
        const item: ResultItem = { cmd, ranges, index: flat.length };
        flat.push(cmd);
        return item;
      }),
    }));
    return { groups, flat };
  });

  let groups = $derived(view.groups);
  let flat = $derived(view.flat);

  $effect(() => {
    if ($commandPaletteOpen) {
      query = "";
      selectedIndex = 0;
      void tick().then(() => inputEl?.focus());
    }
  });

  $effect(() => {
    if (selectedIndex >= flat.length) selectedIndex = Math.max(0, flat.length - 1);
  });

  $effect(() => {
    if (!$commandPaletteOpen) return;
    void selectedIndex;
    void tick().then(() => {
      resultsEl?.querySelector(".result.active")?.scrollIntoView({ block: "nearest" });
    });
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
      case "action.refresh_manifest": void loadCatalog({ trigger: "manual_user" }); break;
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
      e.stopPropagation();
      nextCategory();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (flat.length === 0) return;
      selectedIndex = (selectedIndex + 1) % flat.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (flat.length === 0) return;
      selectedIndex = (selectedIndex - 1 + flat.length) % flat.length;
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const cmd = flat[selectedIndex];
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
    aria-label={$t("component.palette.aria")}
    onclick={onBackdropClick}
    onkeydown={(e) => { if (e.key === "Escape") close(); }}
    tabindex="-1"
    use:focusTrap
  >
    <div class="palette">
      <div class="palette-search">
        <Search class="palette-search-icon" size={18} strokeWidth={2.2} />
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKeydown}
          type="text"
          placeholder={$t("component.palette.placeholder")}
          spellcheck="false"
          autocomplete="off"
        />
        <kbd class="palette-search-kbd">Esc</kbd>
      </div>

      <div class="palette-categories">
        {#each categories as cat}
          <button
            class="category"
            class:active={category === cat}
            onclick={() => { category = cat; selectedIndex = 0; inputEl?.focus(); }}
          >{categoryLabel(cat)}</button>
        {/each}
        <span class="tab-hint">{$t("component.palette.tabToCycle")}</span>
      </div>

      <div class="palette-results" bind:this={resultsEl}>
        {#if flat.length === 0}
          <div class="palette-empty">
            <span class="palette-empty-mark" aria-hidden="true"><Command size={20} strokeWidth={2} /></span>
            <span class="palette-empty-title">{$t("component.palette.empty.title", { q: query.trim() })}</span>
            <span class="palette-empty-hint">{$t("component.palette.empty.hint")}</span>
          </div>
        {:else}
          {#each groups as group (group.key)}
            <div class="result-group" data-cat={group.key}>
              <div class="result-group-head">{group.label}</div>
              {#each group.items as item (item.cmd.id)}
                {@const Icon = ICONS[item.cmd.icon] ?? Command}
                <button
                  class="result"
                  class:active={item.index === selectedIndex}
                  data-cat={item.cmd.category}
                  onclick={() => void runCommand(item.cmd)}
                  onmouseenter={() => { selectedIndex = item.index; }}
                >
                  <span class="result-icon" data-cat={item.cmd.category} aria-hidden="true"><Icon size={16} strokeWidth={2} /></span>
                  <span class="result-text">
                    <span class="result-title">
                      {#each highlightSegments(cmdTitle(item.cmd), item.ranges) as seg}
                        {#if seg.hit}<mark class="result-hl">{seg.text}</mark>{:else}{seg.text}{/if}
                      {/each}
                    </span>
                    {#if item.cmd.hint}
                      <span class="result-hint">{cmdHint(item.cmd)}</span>
                    {/if}
                  </span>
                  {#if item.cmd.shortcut}
                    <span class="result-shortcut" aria-hidden="true">
                      {#each item.cmd.shortcut as key}
                        <kbd>{key.toUpperCase()}</kbd>
                      {/each}
                    </span>
                  {/if}
                  {#if item.index === selectedIndex}
                    <span class="enter-hint" aria-hidden="true">↵</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/each}
        {/if}
      </div>

      <div class="palette-footer">
        <span class="footer-hint"><span class="kbd">↑</span><span class="kbd">↓</span> {$t("component.palette.footer.navigate")}</span>
        <span class="footer-hint"><span class="kbd">↵</span> {$t("component.palette.footer.run")}</span>
        <span class="footer-hint"><span class="kbd">Tab</span> {$t("component.palette.footer.category")}</span>
        <span class="footer-hint"><span class="kbd">Esc</span> {$t("component.palette.footer.close")}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: transparent;
    z-index: 200;
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding: calc(var(--topbar-height) + 6px) 12px 0;
    animation: fadeIn var(--dur-fast) var(--ease-out);
  }
  .palette {
    width: min(460px, calc(100vw - 24px));
    max-width: 560px;
    max-height: 480px;
    background: var(--glass-strong);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
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
    gap: 12px;
    padding: 15px 16px;
    color: var(--text-muted);
  }
  .palette-search :global(.palette-search-icon) { flex-shrink: 0; color: var(--text-muted); }
  .palette-search input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 500;
    letter-spacing: var(--letter-tight);
    padding: 0;
    outline: none;
  }
  .palette-search input:focus { outline: none; border: none; box-shadow: none; }
  .palette-search input::placeholder { color: var(--text-placeholder); font-weight: 400; }
  .palette-search input::selection { background: var(--accent-soft); color: var(--text-primary); }
  .palette-search-kbd {
    flex-shrink: 0;
    font-size: var(--fs-2xs);
    font-weight: 600;
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-card);
  }
  .palette-categories {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 16px 12px;
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
    padding: 6px 8px 8px;
    border-top: 1px solid var(--border);
  }
  .result-group { padding: 4px 0; }
  .result-group-head {
    padding: 8px 12px 4px;
    font-size: var(--fs-2xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .palette-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 40px 16px;
    text-align: center;
  }
  .palette-empty-mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-muted);
    margin-bottom: 4px;
  }
  .palette-empty-title { font-size: var(--fs-sm); font-weight: 600; color: var(--text-primary); }
  .palette-empty-hint { font-size: var(--fs-xs); color: var(--text-muted); }
  .result {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 9px 12px;
    border-radius: var(--radius-lg);
    text-align: left;
    color: var(--text-secondary);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .result.active { background: var(--accent-soft); color: var(--text-primary); }
  .result-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-muted);
  }
  .result-icon[data-cat="navigate"] { color: var(--accent); background: var(--accent-dim); }
  .result-icon[data-cat="action"] { color: var(--badge-green-fg); background: color-mix(in oklab, var(--badge-green-fg) 14%, transparent); }
  .result-icon[data-cat="settings"] { color: var(--badge-purple-fg); background: color-mix(in oklab, var(--badge-purple-fg) 14%, transparent); }
  .result-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .result-title { font-size: var(--fs-sm); font-weight: 500; color: inherit; }
  .result-hl {
    background: transparent;
    color: var(--accent);
    font-weight: 700;
  }
  .result.active .result-hl { color: var(--accent); }
  .result-hint {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .result-shortcut { flex-shrink: 0; display: inline-flex; gap: 3px; }
  .result-shortcut kbd {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: var(--fs-2xs);
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
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
