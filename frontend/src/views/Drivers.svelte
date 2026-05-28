<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    openUrl,
    installDriver,
    DRIVER_INSTALL_EVENT,
    type DriverStatusReport,
    type DriverInstallProgress,
    type InstallStage,
    type GpuVendor,
  } from "../lib/api";
  import { driverStatusLabel, driverStatusTone, sortDriverReports } from "../lib/drivers";
  import {
    driverReports,
    driverCheckInProgress,
    driverCheckError,
    loadDriverUpdates,
    showToast,
  } from "../lib/stores";
  import DlssOverridePanel from "../components/DlssOverridePanel.svelte";
  import DriverHistoryFlyout from "../components/DriverHistoryFlyout.svelte";

  let installingVendor = $state<string | null>(null);
  let installStage = $state<InstallStage | null>(null);
  let installMessage = $state("");
  let installFraction = $state<number | null>(null);
  let expandedModel = $state<string | null>(null);
  let historyTarget = $state<{ vendor: GpuVendor; model: string; accent: string } | null>(null);

  const VENDOR_ACCENT: Record<GpuVendor, string> = {
    nvidia: "#76b900",
    amd: "#ed1c24",
    intel: "#2f9be6",
    other: "#94a3b8",
  };

  const VENDOR_LABEL: Record<GpuVendor, string> = {
    nvidia: "NVIDIA",
    amd: "AMD",
    intel: "Intel",
    other: "GPU",
  };

  let reports = $derived(sortDriverReports($driverReports));
  let nvidiaPacked = $derived(
    $driverReports.find((r) => r.device.vendor === "nvidia")?.installed.packed ?? 0,
  );
  let hasNvidia = $derived($driverReports.some((r) => r.device.vendor === "nvidia"));
  let nonNvidia = $derived(
    $driverReports
      .map((r) => r.device.vendor)
      .filter((v) => v === "amd" || v === "intel") as ("amd" | "intel")[],
  );

  function vendorLabel(vendor: GpuVendor): string {
    return VENDOR_LABEL[vendor] ?? "GPU";
  }

  function sizeLabel(bytes: number): string {
    if (bytes <= 0) return "";
    const mb = bytes / (1024 * 1024);
    return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
  }

  function notesUrl(report: DriverStatusReport): string | null {
    return report.latest?.release_notes_url ?? report.latest?.changelog?.notes_page_url ?? null;
  }

  function canDownload(report: DriverStatusReport): boolean {
    return report.status === "update_available" && !!report.latest?.download_url;
  }

  function hasChangelog(report: DriverStatusReport): boolean {
    const log = report.latest?.changelog;
    return !!log && (log.highlights.length > 0 || log.fixed.length > 0);
  }

  async function openNotes(report: DriverStatusReport): Promise<void> {
    const url = notesUrl(report);
    if (!url) return;
    try {
      await openUrl(url);
    } catch (err) {
      showToast("warning", `Open link failed: ${String(err)}`);
    }
  }

  function toggleChangelog(model: string): void {
    expandedModel = expandedModel === model ? null : model;
  }

  onMount(() => {
    void loadDriverUpdates();
    const unlisten = listen<DriverInstallProgress>(DRIVER_INSTALL_EVENT, (event) => {
      installStage = event.payload.stage;
      installMessage = event.payload.message;
      installFraction = event.payload.progress;
    });
    return () => {
      void unlisten.then((off) => off());
    };
  });

  async function install(report: DriverStatusReport): Promise<void> {
    if (!report.latest?.download_url || installingVendor) return;
    installingVendor = report.device.vendor;
    installStage = "downloading";
    installMessage = "Starting…";
    installFraction = null;
    try {
      const outcome = await installDriver(report.device.vendor, report.latest.download_url);
      if (outcome.stage === "completed") {
        showToast("success", outcome.message);
        await loadDriverUpdates();
      } else if (outcome.stage === "cancelled") {
        showToast("warning", outcome.message);
      } else {
        showToast("danger", outcome.message);
      }
    } catch (err) {
      showToast("danger", `Install failed: ${err}`);
    } finally {
      installingVendor = null;
      installStage = null;
      installFraction = null;
    }
  }
</script>

<section class="drivers-view">
  <header class="view-head">
    <div>
      <h1>Drivers</h1>
      <p class="sub">
        Latest GPU driver for each detected adapter, checked live against the vendor — plus DLSS preset
        and frame-generation controls.
      </p>
    </div>
    <button class="check-btn" onclick={() => loadDriverUpdates()} disabled={$driverCheckInProgress}>
      {$driverCheckInProgress ? "Checking…" : "Check for updates"}
    </button>
  </header>

  {#if $driverCheckError}
    <p class="error-banner">{$driverCheckError}</p>
  {/if}

  {#if reports.length === 0 && !$driverCheckInProgress}
    <p class="empty">No GPUs detected.</p>
  {/if}

  <ul class="driver-list">
    {#each reports as report (report.device.model)}
      {@const tone = driverStatusTone(report.status)}
      {@const expanded = expandedModel === report.device.model}
      {@const showNotes = !!notesUrl(report)}
      <li class="driver-card">
        <div class="card-row">
          <div class="card-main">
            <span class="vendor-pill" data-vendor={report.device.vendor}>
              {vendorLabel(report.device.vendor)}
            </span>
            <div class="model-block">
              <span class="model">{report.device.model}</span>
              <span class="versions mono">
                {report.installed.display}
                {#if report.status === "update_available" && report.latest}
                  <span class="arrow">→</span>
                  <span class="next">{report.latest.version.display}</span>
                  {#if report.latest.is_beta}<span class="chan-chip beta">Beta</span>{:else}<span class="chan-chip whql">WHQL</span>{/if}
                {/if}
              </span>
            </div>
          </div>
          <div class="card-side">
            {#if installingVendor === report.device.vendor}
              <div class="install-live">
                <span class="install-stage">{installStage}</span>
                <div class="install-bar"><div class="install-fill" style:width={`${Math.round((installFraction ?? 0) * 100)}%`}></div></div>
                <span class="install-msg">{installMessage}</span>
              </div>
            {:else}
              {#if canDownload(report)}
                {@const size = sizeLabel(report.latest?.size_bytes ?? 0)}
                <button class="driver-update" onclick={() => install(report)} disabled={!!installingVendor}>
                  <span class="driver-update-label">Update to v{report.latest?.version.display}</span>
                  {#if size}<span class="driver-update-size mono">{size}</span>{/if}
                </button>
              {:else}
                <span class="driver-state" data-tone={tone}>{driverStatusLabel(report.status)}</span>
              {/if}
              <div class="driver-secondary">
                {#if showNotes}
                  <button
                    class="driver-icon"
                    onclick={() => openNotes(report)}
                    title={report.status === "update_available" && !report.latest?.download_url ? "Open the driver download page" : "Open the official release notes"}
                    aria-label="Release notes"
                  >
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                  </button>
                {/if}
                <button
                  class="driver-icon"
                  onclick={() => (historyTarget = { vendor: report.device.vendor, model: report.device.model, accent: VENDOR_ACCENT[report.device.vendor] })}
                  title="Browse every driver version known compatible with this GPU"
                  aria-label="All versions"
                >
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>
                </button>
              </div>
            {/if}
          </div>
        </div>

        {#if hasChangelog(report) || showNotes}
          <button class="changelog-toggle" onclick={() => toggleChangelog(report.device.model)} aria-expanded={expanded}>
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" class="chev" class:open={expanded}><polyline points="6 9 12 15 18 9"/></svg>
            What's new
          </button>
          {#if expanded}
            <div class="changelog">
              {#if hasChangelog(report) && report.latest?.changelog}
                {#if report.latest.changelog.highlights.length > 0}
                  <ul class="cl-highlights">
                    {#each report.latest.changelog.highlights as h}<li>{h}</li>{/each}
                  </ul>
                {/if}
                {#if report.latest.changelog.fixed.length > 0}
                  <span class="cl-label">Fixed</span>
                  <ul class="cl-fixed">
                    {#each report.latest.changelog.fixed as f}<li>{f}</li>{/each}
                  </ul>
                {/if}
              {:else}
                <p class="cl-empty">No inline notes published for this release.</p>
              {/if}
              {#if showNotes}
                <button class="link-btn inline" onclick={() => openNotes(report)}>Full release notes ↗</button>
              {/if}
            </div>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>

  {#if hasNvidia}
    <section class="feature-block">
      <div class="feature-head">
        <h2>DLSS Overrides</h2>
        <span class="vendor-pill" data-vendor="nvidia">NVIDIA</span>
      </div>
      <p class="feature-sub">
        Force the DLSS preset, frame-generation mode and multiplier globally through the NVIDIA driver
        profile — the same mechanism the NVIDIA app uses. Per-game overrides live in each game's detail
        drawer.
      </p>
      <DlssOverridePanel scope={{ scope: "global" }} driverPacked={nvidiaPacked} />
    </section>
  {/if}

  {#if historyTarget}
    <DriverHistoryFlyout
      vendor={historyTarget.vendor}
      model={historyTarget.model}
      accent={historyTarget.accent}
      onClose={() => (historyTarget = null)}
    />
  {/if}

  {#if nonNvidia.length > 0}
    <section class="feature-block muted-block">
      <div class="feature-head">
        <h2>Upscaling on {nonNvidia.includes("amd") ? "AMD" : ""}{nonNvidia.includes("amd") && nonNvidia.includes("intel") ? " / " : ""}{nonNvidia.includes("intel") ? "Intel" : ""}</h2>
      </div>
      <p class="feature-sub">
        Driver-profile preset overrides are NVIDIA-only (NVAPI). On AMD and Intel, FSR and XeSS are
        controlled per game by swapping their DLLs — manage those from the game's detail drawer and the
        Catalog. This tab keeps your AMD / Intel driver itself up to date above.
      </p>
    </section>
  {/if}
</section>

<style>
  .drivers-view { display: flex; flex-direction: column; gap: 20px; }
  .view-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .view-head h1 { font-size: 22px; font-weight: 700; letter-spacing: -0.01em; color: var(--text-primary); }
  .sub { font-size: 13px; color: var(--text-muted); margin-top: 4px; max-width: 60ch; }
  .check-btn {
    height: 38px;
    padding: 0 16px;
    border-radius: var(--radius-lg);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    transition: background var(--dur-fast) var(--ease);
  }
  .check-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .check-btn:disabled { opacity: 0.6; cursor: default; }
  .error-banner {
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    background: var(--danger-dim, rgba(220, 60, 60, 0.12));
    color: var(--danger, #e05b5b);
    font-size: 13px;
  }
  .empty { color: var(--text-muted); font-size: 14px; padding: 24px 0; }
  .driver-list { display: flex; flex-direction: column; gap: 10px; list-style: none; padding: 0; margin: 0; }
  .driver-card {
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
  }
  .card-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .card-main { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .vendor-pill {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
    padding: 4px 9px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .vendor-pill[data-vendor="nvidia"] { color: #76b900; }
  .vendor-pill[data-vendor="amd"] { color: #ed1c24; }
  .vendor-pill[data-vendor="intel"] { color: #2f9be6; }
  .model-block { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .model { font-size: 14px; font-weight: 600; color: var(--text-primary); }
  .versions { font-size: 12px; color: var(--text-muted); font-variant-numeric: tabular-nums; display: inline-flex; align-items: center; gap: 6px; }
  .arrow { color: var(--text-muted); }
  .next { color: var(--accent); font-weight: 600; }
  .chan-chip { font-size: 9px; font-weight: 700; padding: 1px 6px; border-radius: var(--radius-full); letter-spacing: 0.04em; }
  .chan-chip.whql { background: var(--success-dim, rgba(70,180,110,0.14)); color: var(--success, #46b46e); }
  .chan-chip.beta { background: var(--warning-dim, rgba(214,160,50,0.14)); color: var(--warning, #d6a032); }
  .card-side { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .driver-update {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 14px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 12.5px;
    font-weight: 600;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);
    transition: background var(--dur-fast) var(--ease);
  }
  .driver-update:hover:not(:disabled) { background: var(--accent-hover); }
  .driver-update:disabled { opacity: 0.55; cursor: default; }
  .driver-update-label { font-variant-numeric: tabular-nums; }
  .driver-update-size {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    background: rgba(0, 0, 0, 0.18);
    opacity: 0.9;
    font-variant-numeric: tabular-nums;
  }
  .driver-state {
    font-size: 12px;
    font-weight: 600;
    padding: 5px 12px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    color: var(--text-muted);
  }
  .driver-state[data-tone="success"] { background: var(--success-dim, rgba(70, 180, 110, 0.14)); color: var(--success, #46b46e); }
  .driver-state[data-tone="accent"] { background: var(--update-dim, var(--accent-dim)); color: var(--update, var(--accent)); }
  .driver-state[data-tone="warning"] { background: var(--warning-dim, rgba(220, 160, 50, 0.14)); color: var(--warning, #d6a032); }
  .driver-secondary { display: inline-flex; align-items: center; gap: 4px; }
  .driver-icon {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    color: var(--text-muted);
    border: 1px solid transparent;
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .driver-icon:hover { color: var(--text-primary); border-color: var(--border-strong); }
  .link-btn { height: 32px; padding: 0 10px; border-radius: var(--radius-lg); background: var(--bg-elevated); color: var(--text-secondary); font-size: 12px; font-weight: 600; }
  .link-btn:hover { color: var(--text-primary); }
  .link-btn.inline { height: 28px; margin-top: 4px; }
  .changelog-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 12px;
    padding: 4px 2px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .changelog-toggle:hover { color: var(--text-primary); }
  .changelog-toggle .chev { transition: transform 0.15s var(--ease); }
  .changelog-toggle .chev.open { transform: rotate(180deg); }
  .changelog {
    margin-top: 8px;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .cl-highlights { margin: 0 0 8px; padding-left: 18px; }
  .cl-highlights li { font-weight: 600; color: var(--text-primary); }
  .cl-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: var(--letter-wider); color: var(--text-muted); }
  .cl-fixed { margin: 4px 0 0; padding-left: 18px; }
  .cl-fixed li { margin-bottom: 2px; }
  .cl-empty { margin: 0; color: var(--text-muted); }

  .feature-block {
    padding: 18px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .feature-block.muted-block { background: transparent; }
  .feature-head { display: flex; align-items: center; gap: 10px; }
  .feature-head h2 { font-size: 16px; font-weight: 700; color: var(--text-primary); }
  .feature-sub { font-size: 12.5px; color: var(--text-muted); line-height: 1.55; max-width: 70ch; margin: 0; }
  .install-live { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; min-width: 180px; }
  .install-stage { font-size: 11px; font-weight: 600; text-transform: capitalize; color: var(--accent); }
  .install-bar { width: 180px; height: 6px; border-radius: var(--radius-full); background: var(--bg-elevated); overflow: hidden; }
  .install-fill { height: 100%; background: var(--accent); transition: width 0.2s var(--ease); }
  .install-msg { font-size: 10px; color: var(--text-muted); max-width: 220px; text-align: right; }
</style>
