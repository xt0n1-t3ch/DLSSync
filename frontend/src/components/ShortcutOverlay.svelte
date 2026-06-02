<script lang="ts">
  import { shortcutOverlayOpen } from "../lib/stores";
  import { SHORTCUTS, type Shortcut, type ShortcutScope } from "../lib/ux";
  import { t } from "../lib/i18n/index";

  const SCOPE_KEYS: Record<ShortcutScope, string> = {
    global: "global",
    library: "library",
    drawer: "drawer",
    modal: "modal",
    palette: "palette",
  };

  const SHORTCUT_KEYS: Record<string, string> = {
    "Open command palette": "open_command_palette",
    "Show keyboard shortcuts": "show_shortcuts",
    "Focus search": "focus_search",
    "Go to Library": "go_library",
    "Go to Catalog": "go_catalog",
    "Go to Backups": "go_backups",
    "Go to Settings": "go_settings",
    "Go to About": "go_about",
    "Close palette / modal / drawer": "close_overlay",
    "Apply all outdated updates": "apply_all_outdated",
    "Rescan installed games": "rescan",
    "Toggle Grid / List view": "toggle_view",
    "Toggle Compact / Comfy density": "toggle_density",
    "Next feature row": "next_feature",
    "Previous feature row": "prev_feature",
    "Toggle feature selection": "toggle_feature",
    "Open version picker": "open_version_picker",
    "Cancel running apply": "cancel_apply",
    "Cycle category filter": "cycle_category",
    "Run selected command": "run_command",
  };

  function scopeLabel(scope: ShortcutScope): string {
    return $t("component.palette.scope." + SCOPE_KEYS[scope]);
  }

  function shortcutDescription(item: Shortcut): string {
    const key = SHORTCUT_KEYS[item.description];
    return key === undefined ? item.description : $t("shortcut." + key + ".description");
  }

  const KEY_LABELS: Record<string, string> = {
    mod: navigatorMod(),
    arrowup: "↑",
    arrowdown: "↓",
    arrowleft: "←",
    arrowright: "→",
    enter: "↵",
    esc: "Esc",
    escape: "Esc",
    space: "Space",
    tab: "Tab",
  };

  function navigatorMod(): string {
    if (typeof navigator === "undefined") return "Ctrl";
    return navigator.platform.toLowerCase().includes("mac") ? "⌘" : "Ctrl";
  }

  function keyLabel(key: string): string {
    return KEY_LABELS[key] ?? key.toUpperCase();
  }

  let grouped = $derived.by(() => {
    const out: Record<ShortcutScope, Shortcut[]> = {
      global: [], library: [], drawer: [], modal: [], palette: [],
    };
    for (const s of SHORTCUTS) {
      out[s.scope].push(s);
    }
    return out;
  });

  function close(): void {
    shortcutOverlayOpen.set(false);
  }

  function onBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) close();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape" || e.key === "?") {
      e.preventDefault();
      close();
    }
  }
</script>

{#if $shortcutOverlayOpen}
  <div
    class="overlay-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label={$t("component.palette.shortcuts.title")}
    tabindex="-1"
    onclick={onBackdropClick}
    onkeydown={onKeydown}
  >
    <div class="overlay">
      <header>
        <h2>{$t("component.palette.shortcuts.title")}</h2>
        <button class="btn btn-ghost btn-sm" onclick={close} aria-label={$t("component.palette.shortcuts.closeAria")}>
          <span class="kbd">Esc</span>
        </button>
      </header>

      <div class="groups">
        {#each Object.entries(grouped) as [scope, items] (scope)}
          {#if items.length > 0}
            <section class="group">
              <h3>{scopeLabel(scope as ShortcutScope)}</h3>
              <ul>
                {#each items as item, i (i)}
                  <li>
                    <span class="desc">{shortcutDescription(item)}</span>
                    <span class="keys">
                      {#each item.keys as k, ki (ki)}
                        {#if ki > 0}<span class="plus">+</span>{/if}
                        <span class="kbd">{keyLabel(k)}</span>
                      {/each}
                    </span>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay-backdrop {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    z-index: 210;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn var(--dur-fast) var(--ease-out);
    padding: 32px;
  }
  .overlay {
    width: min(840px, 100%);
    max-height: 80vh;
    overflow-y: auto;
    background: var(--glass-strong);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    animation: pop-in var(--dur-fast) var(--ease-out);
    padding: 22px 26px 26px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 18px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  header h2 {
    font-size: var(--fs-xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
  }
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 18px 28px;
  }
  .group h3 {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    margin-bottom: 10px;
  }
  .group ul { list-style: none; display: flex; flex-direction: column; gap: 8px; }
  .group li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: var(--fs-sm);
  }
  .desc { color: var(--text-secondary); }
  .keys { display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; }
  .plus { color: var(--text-muted); font-size: var(--fs-2xs); }
</style>
