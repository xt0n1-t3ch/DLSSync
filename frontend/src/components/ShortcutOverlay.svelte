<script lang="ts">
  import { shortcutOverlayOpen } from "../lib/stores";
  import { SHORTCUTS, type Shortcut, type ShortcutScope } from "../lib/ux";

  const SCOPE_LABELS: Record<ShortcutScope, string> = {
    global: "Global",
    library: "Library",
    drawer: "Game drawer",
    modal: "Modal",
    palette: "Command palette",
  };

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
    aria-label="Keyboard shortcuts"
    tabindex="-1"
    onclick={onBackdropClick}
    onkeydown={onKeydown}
  >
    <div class="overlay">
      <header>
        <h2>Keyboard shortcuts</h2>
        <button class="btn btn-ghost btn-sm" onclick={close} aria-label="Close shortcuts overlay">
          <span class="kbd">Esc</span>
        </button>
      </header>

      <div class="groups">
        {#each Object.entries(grouped) as [scope, items] (scope)}
          {#if items.length > 0}
            <section class="group">
              <h3>{SCOPE_LABELS[scope as ShortcutScope]}</h3>
              <ul>
                {#each items as item, i (i)}
                  <li>
                    <span class="desc">{item.description}</span>
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
