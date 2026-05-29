<script lang="ts">
  import { onMount } from "svelte";
  import {
    applyDlssOverride,
    resetDlssOverride,
    readDlssOverrideConfig,
    openUrl,
    type OverrideScope,
    type DlssOverrideConfig,
    type DlssOverrideSource,
    type DlssPreset,
    type FrameGenMode,
    type FrameGenCount,
  } from "../lib/api";
  import {
    SR_PRESET_OPTIONS,
    FG_MODE_OPTIONS,
    FG_COUNT_OPTIONS,
    emptyDlssConfig,
    dlss4Available,
    dynamicMfgAvailable,
  } from "../lib/dlss";
  import { showToast } from "../lib/stores";
  import Checkbox from "./Checkbox.svelte";
  import Select from "./Select.svelte";

  let srSelectOptions = $derived<{ value: DlssPreset | null; label: string }[]>([
    { value: null, label: "No preset override" },
    ...SR_PRESET_OPTIONS.map((o) => ({ value: o.value, label: o.label })),
  ]);
  let fgModeSelectOptions = $derived<
    { value: FrameGenMode | null; label: string; disabled?: boolean }[]
  >([
    { value: null, label: "No mode override" },
    ...FG_MODE_OPTIONS.map((o) => ({
      value: o.value,
      label:
        o.value === "dynamic" && !dynamicOk ? `${o.label} — needs driver 595.97+` : o.label,
      disabled: o.value === "dynamic" && !dynamicOk,
    })),
  ]);
  let fgCountSelectOptions = $derived<{ value: FrameGenCount | null; label: string }[]>([
    { value: null, label: "App controlled" },
    ...FG_COUNT_OPTIONS.filter((o) => o.value !== "app_controlled").map((o) => ({
      value: o.value,
      label: o.label,
    })),
  ]);

  let { scope, driverPacked = 0 }: { scope: OverrideScope; driverPacked?: number } = $props();

  let config = $state<DlssOverrideConfig>(emptyDlssConfig());
  let busy = $state(false);
  let activeCount = $state(0);
  let source = $state<DlssOverrideSource>("none");
  let sourceLabel = $derived(
    source === "none"
      ? null
      : source === "per_game"
        ? "From NVIDIA driver"
        : scope.scope === "global"
          ? "Set in NVIDIA driver"
          : "Inherited from Global default",
  );

  let dlss4Ok = $derived(driverPacked === 0 || dlss4Available(driverPacked));
  let dynamicOk = $derived(driverPacked === 0 || dynamicMfgAvailable(driverPacked));

  let srHelp = $derived(SR_PRESET_OPTIONS.find((o) => o.value === config.sr_preset) ?? null);
  let fgModeHelp = $derived(FG_MODE_OPTIONS.find((o) => o.value === config.fg_mode) ?? null);
  let fgCountHelp = $derived(FG_COUNT_OPTIONS.find((o) => o.value === config.fg_fixed_count) ?? null);

  async function refresh(): Promise<void> {
    try {
      const readback = await readDlssOverrideConfig(scope);
      config = readback.config;
      activeCount = readback.active_count;
      source = readback.source;
    } catch {
      activeCount = 0;
      source = "none";
    }
  }

  onMount(refresh);

  async function learnMore(url: string): Promise<void> {
    try {
      await openUrl(url);
    } catch (err) {
      showToast("warning", `Open link failed: ${String(err)}`);
    }
  }

  async function apply(): Promise<void> {
    busy = true;
    try {
      const outcome = await applyDlssOverride(scope, config);
      if (outcome.needs_elevation) {
        showToast(
          "warning",
          "Super Resolution applied. Frame-generation overrides need administrator — relaunch DLSSync as administrator to apply them.",
        );
      } else {
        showToast("success", "DLSS override applied — restart the game to take effect.");
      }
      await refresh();
    } catch (err) {
      showToast("danger", `Apply failed: ${err}`);
    } finally {
      busy = false;
    }
  }

  async function reset(): Promise<void> {
    busy = true;
    try {
      await resetDlssOverride(scope);
      config = emptyDlssConfig();
      showToast("success", "DLSS override reset to driver default.");
      await refresh();
    } catch (err) {
      showToast("danger", `Reset failed: ${err}`);
    } finally {
      busy = false;
    }
  }
</script>

<div class="dlss">
  <div class="dlss-head">
    <h4>DLSS Overrides{scope.scope === "global" ? " — Global default" : ""}</h4>
    {#if activeCount > 0}<span class="dlss-active">{activeCount} active</span>{/if}
    {#if sourceLabel}<span class="dlss-source">{sourceLabel}</span>{/if}
    <button class="dlss-refresh" onclick={refresh} title="Re-read the current values from the NVIDIA driver">
      Refresh from driver
    </button>
  </div>

  {#if !dlss4Ok}
    <p class="dlss-warn">Game Ready Driver 572.16 or newer is required for DLSS 4 overrides.</p>
  {/if}

  <section class="dlss-group">
    <span class="dlss-group-title">Super Resolution</span>
    <Checkbox bind:checked={config.enable_sr_dll_override} label="Force the latest installed DLSS DLL" />
    <div class="dlss-field">
      <span class="dlss-field-label">Model preset</span>
      <div class="dlss-control">
        <Select
          bind:value={config.sr_preset}
          options={srSelectOptions}
          placeholder="No preset override"
          ariaLabel="DLSS model preset"
        />
      </div>
    </div>
    {#if srHelp}
      <p class="dlss-help">
        {srHelp.description}
        <button class="dlss-learn" onclick={() => learnMore(srHelp.sourceUrl)}>Learn more ↗</button>
      </p>
    {/if}
  </section>

  <section class="dlss-group">
    <span class="dlss-group-title">Frame Generation</span>
    <Checkbox
      bind:checked={config.enable_fg_dll_override}
      label="Force the latest installed Frame Generation DLL"
    />
    <div class="dlss-field">
      <span class="dlss-field-label">Mode</span>
      <div class="dlss-control">
        <Select
          bind:value={config.fg_mode}
          options={fgModeSelectOptions}
          placeholder="No mode override"
          ariaLabel="Frame generation mode"
        />
      </div>
    </div>
    {#if fgModeHelp}
      <p class="dlss-help">
        {fgModeHelp.description}
        <button class="dlss-learn" onclick={() => learnMore(fgModeHelp.sourceUrl)}>Learn more ↗</button>
      </p>
    {/if}

    {#if config.fg_mode === "fixed"}
      <div class="dlss-field">
        <span class="dlss-field-label">Fixed multiplier</span>
        <div class="dlss-control">
          <Select
            bind:value={config.fg_fixed_count}
            options={fgCountSelectOptions}
            placeholder="App controlled"
            ariaLabel="Fixed multiplier"
          />
        </div>
      </div>
      {#if fgCountHelp}
        <p class="dlss-help">
          {fgCountHelp.description}
          <button class="dlss-learn" onclick={() => learnMore(fgCountHelp.sourceUrl)}>Learn more ↗</button>
        </p>
      {/if}
    {/if}

    {#if config.fg_mode === "dynamic"}
      <div class="dlss-field">
        <span class="dlss-field-label">Target frame rate</span>
        <input
          class="dlss-input"
          type="number"
          min="30"
          max="1000"
          bind:value={config.fg_dynamic_target_fps}
          placeholder="e.g. 240"
        />
      </div>
      <p class="dlss-help">
        Dynamic targets this frame rate (or your monitor's max refresh). Leave blank to match the
        display. Not compatible with frame-rate limiters or V-Sync.
      </p>
    {/if}
  </section>

  <div class="dlss-actions">
    <button class="dlss-apply" onclick={apply} disabled={busy}>{busy ? "Working…" : "Apply"}</button>
    <button class="dlss-reset" onclick={reset} disabled={busy}>Reset to default</button>
  </div>
  <p class="dlss-note">
    Writes an NVIDIA driver application profile — the same mechanism the NVIDIA app uses. Fully
    reversible with Reset. Multiplayer titles with anti-cheat may flag a forced profile; check the
    game's policy first.
  </p>
</div>

<style>
  .dlss {
    display: flex;
    flex-direction: column;
    gap: 14px;
    container-type: inline-size;
  }
  .dlss-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dlss-head h4 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
  }
  .dlss-active {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--update-dim, var(--accent-dim));
    color: var(--update, var(--accent));
  }
  .dlss-source {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }
  .dlss-refresh {
    margin-left: auto;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    background: transparent;
    white-space: nowrap;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .dlss-refresh:hover { color: var(--accent); background: var(--accent-dim); }
  .dlss-refresh:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .dlss-warn {
    font-size: 12px;
    color: var(--warning, #d6a032);
    background: var(--warning-dim, rgba(214, 160, 50, 0.12));
    border: 1px solid var(--warning, #d6a032);
    border-radius: var(--radius-md);
    padding: 8px 12px;
  }
  .dlss-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
  }
  .dlss-group-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
  }
  .dlss-control {
    flex: 0 0 auto;
    width: 240px;
    max-width: 60%;
  }
  .dlss-field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .dlss-field-label {
    font-size: 13px;
    color: var(--text-secondary);
    flex: 1 1 auto;
    min-width: 0;
  }
  .dlss-input {
    flex: 0 0 auto;
    width: 240px;
    max-width: 60%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--radius-md, 8px);
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border);
    font-size: 13px;
  }
  .dlss-help {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
    margin: 0;
  }
  .dlss-learn {
    display: inline;
    padding: 0;
    margin-left: 4px;
    background: none;
    border: none;
    color: var(--accent);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .dlss-learn:hover {
    text-decoration: underline;
  }
  .dlss-actions {
    display: flex;
    gap: 10px;
  }
  .dlss-apply,
  .dlss-reset {
    height: 36px;
    padding: 0 16px;
    border-radius: var(--radius-lg);
    font-size: 13px;
    font-weight: 600;
  }
  .dlss-apply {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .dlss-apply:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .dlss-reset {
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }
  .dlss-reset:hover:not(:disabled) {
    color: var(--text-primary);
  }
  .dlss-apply:disabled,
  .dlss-reset:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .dlss-note {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
    margin: 0;
  }
  @container (max-width: 380px) {
    .dlss-field {
      flex-direction: column;
      align-items: stretch;
    }
    .dlss-control,
    .dlss-input {
      width: 100%;
      max-width: none;
    }
  }
</style>
