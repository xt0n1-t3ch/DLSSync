<script lang="ts">
  import { onMount } from "svelte";
  import { setEfficiencyMode } from "../lib/api";
  import {
    EFFICIENCY_PREF_EVENT,
    EFFICIENCY_PREF_KEY,
    readEfficiencyPreference,
    type EfficiencyPreferenceEvent,
  } from "../lib/efficiencyMode";

  const EFFICIENCY_REASSERT_MS = 15_000;

  let efficiencyEnabled = false;
  let efficiencyApplied = false;
  let reassertTimer: ReturnType<typeof setInterval> | undefined;

  onMount(() => {
    try {
      efficiencyEnabled = readEfficiencyPreference();
    } catch {}
    if (efficiencyEnabled) {
      void enableEfficiency();
    }

    const onPreference = (event: Event): void => {
      const enabled = (event as EfficiencyPreferenceEvent).detail?.enabled;
      if (typeof enabled === "boolean") {
        void setEnabled(enabled);
      }
    };
    const onStorage = (event: StorageEvent): void => {
      if (event.key === EFFICIENCY_PREF_KEY) {
        void setEnabled(event.newValue !== "false");
      }
    };

    window.addEventListener(EFFICIENCY_PREF_EVENT, onPreference);
    window.addEventListener("storage", onStorage);

    return () => {
      window.removeEventListener(EFFICIENCY_PREF_EVENT, onPreference);
      window.removeEventListener("storage", onStorage);
      stopEfficiencyPulse();
      if (efficiencyApplied) {
        void setEfficiencyMode(false);
      }
    };
  });

  async function setEnabled(enabled: boolean): Promise<void> {
    efficiencyEnabled = enabled;
    if (enabled) {
      await enableEfficiency();
    } else {
      await disableEfficiency();
    }
  }

  async function enableEfficiency(): Promise<void> {
    stopEfficiencyPulse();
    await applyEfficiency(true, { force: true });
    reassertTimer = setInterval(() => {
      if (efficiencyEnabled) {
        void applyEfficiency(true, { force: true });
      }
    }, EFFICIENCY_REASSERT_MS);
  }

  async function disableEfficiency(): Promise<void> {
    stopEfficiencyPulse();
    await applyEfficiency(false);
  }

  function stopEfficiencyPulse(): void {
    if (reassertTimer) {
      clearInterval(reassertTimer);
      reassertTimer = undefined;
    }
  }

  async function applyEfficiency(enable: boolean, opts: { force?: boolean } = {}): Promise<void> {
    if (!opts.force && efficiencyApplied === enable) return;
    try {
      await setEfficiencyMode(enable);
      efficiencyApplied = enable;
    } catch {}
  }
</script>
