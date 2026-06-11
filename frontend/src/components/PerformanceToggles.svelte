<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n/index";
  import {
    EFFICIENCY_PREF_EVENT,
    EFFICIENCY_PREF_KEY,
    readEfficiencyPreference,
    writeEfficiencyPreference,
    type EfficiencyPreferenceEvent,
  } from "../lib/efficiencyMode";
  import Sparkles from "@lucide/svelte/icons/sparkles";

  let efficiencyEnabled = $state(true);

  onMount(() => {
    try {
      efficiencyEnabled = readEfficiencyPreference();
    } catch {}

    const onPreference = (event: Event): void => {
      const enabled = (event as EfficiencyPreferenceEvent).detail?.enabled;
      if (typeof enabled === "boolean") {
        efficiencyEnabled = enabled;
      }
    };
    const onStorage = (event: StorageEvent): void => {
      if (event.key === EFFICIENCY_PREF_KEY) {
        efficiencyEnabled = event.newValue !== "false";
      }
    };

    window.addEventListener(EFFICIENCY_PREF_EVENT, onPreference);
    window.addEventListener("storage", onStorage);

    return () => {
      window.removeEventListener(EFFICIENCY_PREF_EVENT, onPreference);
      window.removeEventListener("storage", onStorage);
    };
  });

  function toggleEfficiency(next: boolean): void {
    efficiencyEnabled = next;
    try { writeEfficiencyPreference(next); } catch {}
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
      <input type="checkbox" checked={efficiencyEnabled} onchange={(e) => toggleEfficiency((e.target as HTMLInputElement).checked)} />
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
