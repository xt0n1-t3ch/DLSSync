<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n/index";
  import { setEfficiencyMode } from "../lib/api";
  import Sparkles from "@lucide/svelte/icons/sparkles";

  const PREF_KEYS = {
    efficiencyOnMinimize: "dlssync-pref-efficiency-on-minimize",
  } as const;

  let efficiencyOnMinimize = $state(true);
  let efficiencyApplied = $state(false);

  onMount(async () => {
    try {
      efficiencyOnMinimize = localStorage.getItem(PREF_KEYS.efficiencyOnMinimize) !== "false";
    } catch {}
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

  async function toggleEfficiency(next: boolean): Promise<void> {
    efficiencyOnMinimize = next;
    try { localStorage.setItem(PREF_KEYS.efficiencyOnMinimize, String(next)); } catch {}
    if (next) {
      await wireEfficiencyHooks();
    } else {
      await teardownEfficiencyHooks();
    }
  }

</script>

<div class="perf-card">
  <div class="perf-row">
    <div class="perf-icon" aria-hidden="true">
      <Sparkles size={14} />
    </div>
    <div class="perf-meta">
      <span class="perf-label">
        {$t("component.perf.efficiency.label")}
        <span class="chip chip-success small-pill">EcoQoS</span>
      </span>
      <span class="perf-sub">{$t("component.perf.efficiency.sub")}</span>
    </div>
    <label class="toggle">
      <input type="checkbox" checked={efficiencyOnMinimize} onchange={(e) => toggleEfficiency((e.target as HTMLInputElement).checked)} />
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
