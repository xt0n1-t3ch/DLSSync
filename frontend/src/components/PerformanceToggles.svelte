<script lang="ts">
  import { onMount } from "svelte";
  import { showToast } from "../lib/stores";
  import { setCloseToTray, getCloseToTray, setEfficiencyMode } from "../lib/api";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import Power from "@lucide/svelte/icons/power";

  const AUTOSTART_ARGS = ["--minimized"];
  const PREF_KEYS = {
    efficiencyOnMinimize: "dlssync-pref-efficiency-on-minimize",
    closeToTray: "dlssync-pref-close-to-tray",
  } as const;

  let closeToTray = $state(true);
  let efficiencyOnMinimize = $state(true);
  let autostartEnabled = $state(false);
  let autostartReady = $state(false);
  let efficiencyApplied = $state(false);

  onMount(async () => {
    try {
      closeToTray = await getCloseToTray();
    } catch {
      closeToTray = true;
    }
    try {
      efficiencyOnMinimize = localStorage.getItem(PREF_KEYS.efficiencyOnMinimize) !== "false";
    } catch {}
    try {
      const { isEnabled } = await import("@tauri-apps/plugin-autostart");
      autostartEnabled = await isEnabled();
      autostartReady = true;
    } catch {
      autostartReady = false;
    }
    if (efficiencyOnMinimize) {
      await wireEfficiencyHooks();
    }
  });

  let cleanupHooks: Array<() => void> = [];

  async function wireEfficiencyHooks(): Promise<void> {
    await teardownEfficiencyHooks();
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const unlistenFocus = await win.onFocusChanged(async (e) => {
        if (e.payload) {
          await applyEfficiency(false);
        }
      });
      cleanupHooks.push(() => unlistenFocus());

      const onVis = (): void => {
        void applyEfficiency(document.visibilityState === "hidden");
      };
      document.addEventListener("visibilitychange", onVis);
      cleanupHooks.push(() => document.removeEventListener("visibilitychange", onVis));
    } catch (err: unknown) {
      console.warn("efficiency hooks unavailable", err);
    }
  }

  async function teardownEfficiencyHooks(): Promise<void> {
    for (const fn of cleanupHooks) {
      try { fn(); } catch {}
    }
    cleanupHooks = [];
    if (efficiencyApplied) {
      try { await setEfficiencyMode(false); } catch {}
      efficiencyApplied = false;
    }
  }

  async function applyEfficiency(enable: boolean): Promise<void> {
    if (efficiencyApplied === enable) return;
    try {
      await setEfficiencyMode(enable);
      efficiencyApplied = enable;
    } catch {}
  }

  async function toggleCloseToTray(next: boolean): Promise<void> {
    closeToTray = next;
    try {
      await setCloseToTray(next);
      try { localStorage.setItem(PREF_KEYS.closeToTray, String(next)); } catch {}
    } catch (err: unknown) {
      showToast("danger", `Set close-to-tray failed: ${String(err)}`);
    }
  }

  async function toggleEfficiency(next: boolean): Promise<void> {
    efficiencyOnMinimize = next;
    try { localStorage.setItem(PREF_KEYS.efficiencyOnMinimize, String(next)); } catch {}
    if (next) {
      await wireEfficiencyHooks();
    } else {
      await teardownEfficiencyHooks();
    }
  }

  async function toggleAutostart(next: boolean): Promise<void> {
    if (!autostartReady) return;
    autostartEnabled = next;
    try {
      const mod = await import("@tauri-apps/plugin-autostart");
      if (next) {
        await mod.enable();
      } else {
        await mod.disable();
      }
      const verified = await mod.isEnabled();
      autostartEnabled = verified;
    } catch (err: unknown) {
      autostartEnabled = !next;
      showToast("danger", `Autostart toggle failed: ${String(err)}`);
    }
  }
</script>

<div class="perf-card">
  <div class="perf-row">
    <div class="perf-icon" aria-hidden="true">
      <Minimize2 size={14} />
    </div>
    <div class="perf-meta">
      <span class="perf-label">Close to tray</span>
      <span class="perf-sub">Keep DLSSync running in the system tray when you click the X. Use the tray icon to bring it back. Recommended for background update checks.</span>
    </div>
    <label class="toggle">
      <input type="checkbox" checked={closeToTray} onchange={(e) => toggleCloseToTray((e.target as HTMLInputElement).checked)} />
      <span class="toggle-slider"></span>
    </label>
  </div>

  <div class="perf-row">
    <div class="perf-icon" aria-hidden="true">
      <Sparkles size={14} />
    </div>
    <div class="perf-meta">
      <span class="perf-label">
        Efficiency Mode when minimized
        <span class="chip chip-success small-pill">EcoQoS</span>
      </span>
      <span class="perf-sub">Schedules DLSSync onto efficient cores and lowers priority while you're not looking at it. Drops CPU/battery footprint near zero. Shows the green leaf badge in Task Manager.</span>
    </div>
    <label class="toggle">
      <input type="checkbox" checked={efficiencyOnMinimize} onchange={(e) => toggleEfficiency((e.target as HTMLInputElement).checked)} />
      <span class="toggle-slider"></span>
    </label>
  </div>

  <div class="perf-row">
    <div class="perf-icon" aria-hidden="true">
      <Power size={14} />
    </div>
    <div class="perf-meta">
      <span class="perf-label">
        Start with Windows
        {#if !autostartReady}<span class="chip chip-neutral small-pill">unavailable</span>{/if}
      </span>
      <span class="perf-sub">Launch DLSSync in the system tray on Windows startup, so update checks happen in the background — no visible window. Launches with <span class="mono">{AUTOSTART_ARGS.join(" ")}</span>.</span>
    </div>
    <label class="toggle">
      <input type="checkbox" checked={autostartEnabled} disabled={!autostartReady} onchange={(e) => toggleAutostart((e.target as HTMLInputElement).checked)} />
      <span class="toggle-slider"></span>
    </label>
  </div>
</div>

<style>
  .perf-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 4px 18px;
  }
  .perf-row {
    display: grid;
    grid-template-columns: 32px 1fr auto;
    gap: 14px;
    align-items: center;
    padding: 14px 0;
  }
  .perf-row + .perf-row { border-top: 1px solid var(--border); }
  .perf-icon {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .perf-meta { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .perf-label {
    font-size: var(--fs-base);
    font-weight: 500;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .perf-sub {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    line-height: var(--lh-snug);
  }
</style>
