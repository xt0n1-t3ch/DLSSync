<script lang="ts">
  import { onMount } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import {
    manifestUpdatedAt,
    catalogVendors,
    catalogStatus,
    games,
    backups,
    showToast,
  } from "../lib/stores";
  import {
    openPath,
    revealPath,
    getAppPaths,
    getSystemInfo,
    buildIssueReport,
    type AppPathsDto,
    type SystemInfo,
  } from "../lib/api";
  import { vendorLabel, vendorAccent } from "../lib/labels";
  import { EXTERNAL_URLS } from "../lib/ux";
  import { fetchStarCount, shareDlssync } from "../lib/community";
  import changelogRaw from "../../../CHANGELOG.md?raw";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Bug from "@lucide/svelte/icons/bug";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import Signature from "@lucide/svelte/icons/signature";
  import History from "@lucide/svelte/icons/history";
  import Cpu from "@lucide/svelte/icons/cpu";
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import MemoryStick from "@lucide/svelte/icons/memory-stick";
  import Monitor from "@lucide/svelte/icons/monitor";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FileText from "@lucide/svelte/icons/file-text";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import Database from "@lucide/svelte/icons/database";
  import Star from "@lucide/svelte/icons/star";
  import Share2 from "@lucide/svelte/icons/share-2";
  import BrandMark from "../components/BrandMark.svelte";
  import NexusLogo from "../components/NexusLogo.svelte";

  let version = $state("dev");
  let appPaths = $state<AppPathsDto | null>(null);
  let systemInfo = $state<SystemInfo | null>(null);
  let systemInfoFailed = $state(false);
  let updateChecking = $state(false);
  let updateMessage = $state<{ kind: "info" | "success" | "warning" | "danger"; text: string } | null>(null);
  let starCount = $state<number | null>(null);

  type ReleaseHighlights = { version: string; bullets: string[] };
  const releaseHighlights = parseLatestRelease(changelogRaw);

  function parseLatestRelease(raw: string): ReleaseHighlights | null {
    const headRe = /^##\s*\[([^\]]+)\]/m;
    const head = raw.match(headRe);
    if (!head) return null;
    const start = (head.index ?? 0) + head[0].length;
    const restRe = /\n##\s*\[/g;
    restRe.lastIndex = start;
    const next = restRe.exec(raw);
    const end = next ? next.index : raw.length;
    const body = raw.slice(start, end);
    const bulletRe = /^\s*[-*]\s+(.+)$/gm;
    const bullets: string[] = [];
    for (const m of body.matchAll(bulletRe)) {
      const text = m[1].trim().replace(/`([^`]+)`/g, "$1").replace(/\*\*([^*]+)\*\*/g, "$1");
      if (text.length > 0) bullets.push(text);
      if (bullets.length >= 6) break;
    }
    return { version: head[1], bullets };
  }

  async function openReleases(): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(EXTERNAL_URLS.releases);
    } catch (err) { showToast("warning", `Open link failed: ${String(err)}`); }
  }

  const familyTween = new Tween(0, { duration: 600, easing: cubicOut });
  const releaseTween = new Tween(0, { duration: 800, easing: cubicOut });
  const gameTween = new Tween(0, { duration: 500, easing: cubicOut });
  const backupTween = new Tween(0, { duration: 500, easing: cubicOut });

  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      version = await getVersion();
    } catch {
      version = "dev";
    }
    try {
      appPaths = await getAppPaths();
    } catch {
      appPaths = null;
    }
    try {
      systemInfo = await getSystemInfo();
    } catch {
      systemInfo = null;
      systemInfoFailed = true;
    }
    try {
      starCount = await fetchStarCount();
    } catch {
      starCount = null;
    }
  });

  async function shareApp(): Promise<void> {
    const result = await shareDlssync();
    if (result === "copied") showToast("success", "Link copied - share DLSSync with a friend");
    else if (result === "failed") showToast("warning", "Could not copy the link");
  }

  function fmtBytes(n: number | null | undefined): string {
    if (n == null || n === 0) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    let v = n;
    while (v >= 1024 && i < units.length - 1) {
      v /= 1024;
      i += 1;
    }
    return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v).toString()} ${units[i]}`;
  }

  let familyCount = $derived(
    $catalogVendors.reduce((acc, v) => acc + v.families.length, 0),
  );
  let releaseCount = $derived(
    $catalogVendors.reduce(
      (acc, v) => acc + v.families.reduce((a, f) => a + f.releaseCount, 0),
      0,
    ),
  );
  let vendorCount = $derived($catalogVendors.length);
  let gameCount = $derived($games.length);
  let backupCount = $derived($backups.length);

  $effect(() => { familyTween.target = familyCount; });
  $effect(() => { releaseTween.target = releaseCount; });
  $effect(() => { gameTween.target = gameCount; });
  $effect(() => { backupTween.target = backupCount; });

  const SOURCES = [
    { vendor: "NVIDIA", url: "https://github.com/NVIDIA/DLSS", label: "DLSS SDK", accent: vendorAccent("nvidia") },
    { vendor: "NVIDIA", url: "https://github.com/NVIDIA-RTX/Streamline", label: "Streamline", accent: vendorAccent("nvidia") },
    { vendor: "NVIDIA", url: "https://github.com/NVIDIA-RTX/REFLEX", label: "Reflex", accent: vendorAccent("nvidia") },
    { vendor: "Intel", url: "https://github.com/intel/xess", label: "XeSS SDK", accent: vendorAccent("intel") },
    { vendor: "AMD", url: "https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK", label: "FidelityFX SDK", accent: vendorAccent("amd") },
    { vendor: "Microsoft", url: "https://github.com/microsoft/DirectStorage", label: "DirectStorage", accent: vendorAccent("microsoft") },
  ];

  const TRADEMARKS_LINE = "DLSS, NVIDIA, GeForce, RTX, Reflex, Streamline are trademarks of NVIDIA Corporation. XeSS, Xe, Arc are trademarks of Intel Corporation. FidelityFX, FSR, Radeon are trademarks of Advanced Micro Devices, Inc. DirectStorage, DirectX, Windows are trademarks of Microsoft Corporation.";

  const REPO_URL = "https://github.com/xt0n1-t3ch/DLSSync";
  const ISSUES_URL = "https://github.com/xt0n1-t3ch/DLSSync/issues";

  async function openExternal(url: string): Promise<void> {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch {
      window.open(url, "_blank");
    }
  }

  async function checkForUpdates(): Promise<void> {
    if (updateChecking) return;
    updateChecking = true;
    updateMessage = { kind: "info", text: "Checking GitHub Releases…" };
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update && (update as { available?: boolean }).available !== false) {
        const next = (update as { version?: string }).version ?? "unknown";
        updateMessage = { kind: "success", text: `Update available: v${next}. Visit the releases page to download.` };
      } else {
        updateMessage = { kind: "success", text: `You're on the latest version (v${version}).` };
      }
    } catch (err: unknown) {
      const msg = String(err);
      updateMessage = {
        kind: "warning",
        text: `Update check failed: ${msg}. The endpoint may be unreachable or no release is published yet.`,
      };
    } finally {
      updateChecking = false;
    }
  }

  async function revealConfig(): Promise<void> {
    if (!appPaths) return;
    try {
      await revealPath(appPaths.settings_file);
    } catch {
      try { await openPath(appPaths.settings_dir); }
      catch (err) { showToast("danger", `Open failed: ${String(err)}`); }
    }
  }
  async function openBackups(): Promise<void> {
    if (!appPaths) return;
    try { await openPath(appPaths.backups_dir); }
    catch (err) { showToast("danger", `Open failed: ${String(err)}`); }
  }
  async function openLogs(): Promise<void> {
    if (!appPaths) return;
    try { await openPath(appPaths.logs_dir); }
    catch {
      try { await openPath(appPaths.root); }
      catch (err) { showToast("danger", `Open failed: ${String(err)}`); }
    }
  }
  async function openRoot(): Promise<void> {
    if (!appPaths) return;
    try { await openPath(appPaths.root); }
    catch (err) { showToast("danger", `Open failed: ${String(err)}`); }
  }

  let reporting = $state(false);
  async function reportBug(): Promise<void> {
    if (reporting) return;
    reporting = true;
    try {
      const report = await buildIssueReport();
      await openExternal(report.url);
    } catch (err: unknown) {
      await openExternal(ISSUES_URL);
      showToast("warning", `Opened a blank issue — diagnostics unavailable: ${String(err)}`);
    } finally {
      reporting = false;
    }
  }
</script>

<header class="view-header">
  <div>
    <h1 class="view-title">About</h1>
    <p class="view-subtitle">DLSSync keeps DLSS, FSR and XeSS technologies synchronized with vendor releases — hash-verified, vendor-signed, fully reversible.</p>
  </div>
  <div class="header-actions">
    <button class="btn btn-ghost" onclick={() => openExternal(REPO_URL)}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .3a12 12 0 0 0-3.79 23.4c.6.11.82-.26.82-.58v-2c-3.34.73-4.04-1.61-4.04-1.61-.55-1.4-1.34-1.77-1.34-1.77-1.1-.75.08-.73.08-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.84 2.81 1.31 3.5 1 .1-.78.42-1.31.76-1.61-2.66-.3-5.46-1.33-5.46-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.12-3.17 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.25 2.87.12 3.17.77.84 1.24 1.91 1.24 3.22 0 4.61-2.8 5.62-5.47 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .3Z"/></svg>
      GitHub
    </button>
    <button class="btn btn-ghost sponsor-btn" onclick={() => openExternal(EXTERNAL_URLS.sponsor)} title="GitHub Sponsors — recurring or one-time support">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.27 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.77-3.4 6.86-8.55 11.53L12 21.35z"/></svg>
      Sponsor
    </button>
    <button class="btn btn-ghost kofi-btn" onclick={() => openExternal(EXTERNAL_URLS.kofi)} title="Ko-fi — quick one-time tip">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17 8h1a4 4 0 0 1 0 8h-1"/><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"/><line x1="6" y1="2" x2="6" y2="4"/><line x1="10" y1="2" x2="10" y2="4"/><line x1="14" y1="2" x2="14" y2="4"/></svg>
      Ko-fi
    </button>
    <button class="btn btn-ghost nexus-btn" onclick={() => openExternal(EXTERNAL_URLS.nexusMod)} title="DLSSync on Nexus Mods — endorse it to boost visibility">
      <NexusLogo size={15} />
      Nexus
    </button>
    <button class="btn btn-ghost" onclick={reportBug} disabled={reporting} title="Open a pre-filled GitHub issue with app version, OS, and recent logs attached">
      {#if reporting}
        <span class="spin"></span>
        Preparing
      {:else}
        <Bug size={14} />
        Report a problem
      {/if}
    </button>
    <button class="btn btn-primary" onclick={checkForUpdates} disabled={updateChecking}>
      {#if updateChecking}
        <span class="spin"></span>
        Checking
      {:else}
        <RefreshCw size={14} />
        Check for updates
      {/if}
    </button>
  </div>
</header>

<section class="brand-row edge-accent" in:fly={{ y: 6, duration: 240 }}>
  <div class="brand-mark" aria-hidden="true">
    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 12a9 9 0 1 0 3-6.7"/>
      <polyline points="3 4 3 9 8 9"/>
    </svg>
  </div>
  <div class="brand-text">
    <div class="brand-title-row">
      <h2 class="brand-title">DLSSync</h2>
      <span class="brand-version mono">v{version}</span>
      <span class="brand-tagline">SYNC · VERIFY · APPLY</span>
    </div>
  </div>
  <div class="brand-pillars">
    <span class="pillar" title="SHA-256 + per-DLL hash verified before apply">
      <ShieldCheck size={13} />
      Hash-verified
    </span>
    <span class="pillar" title="Authenticode publisher gate enforced">
      <Signature size={13} />
      Vendor-signed
    </span>
    <span class="pillar" title="Auto-backup, one-click restore">
      <History size={13} />
      Reversible
    </span>
  </div>
</section>

{#if releaseHighlights && releaseHighlights.bullets.length > 0}
  <section class="whats-new" in:fly={{ y: 6, duration: 240, delay: 40 }}>
    <header class="wn-head">
      <span class="wn-eyebrow">What's new</span>
      <span class="wn-version mono">v{releaseHighlights.version}</span>
      <button class="wn-link" onclick={openReleases} title="Open GitHub releases">View full changelog →</button>
    </header>
    <ul class="wn-list">
      {#each releaseHighlights.bullets as bullet (bullet)}
        <li>{bullet}</li>
      {/each}
    </ul>
  </section>
{/if}

{#if updateMessage}
  <div
    class="update-banner"
    class:is-success={updateMessage.kind === "success"}
    class:is-warning={updateMessage.kind === "warning"}
    class:is-danger={updateMessage.kind === "danger"}
    class:is-info={updateMessage.kind === "info"}
    in:fly={{ y: -4, duration: 180 }}
  >
    {updateMessage.text}
  </div>
{/if}

<section class="info-bar" in:fly={{ y: 6, duration: 280, delay: 60 }}>
  <div class="info-item">
    <span class="info-item-label">Families tracked</span>
    <span class="info-item-value">{Math.round(familyTween.current)}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Versions in manifest</span>
    <span class="info-item-value">{Math.round(releaseTween.current).toLocaleString()}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Upstream vendors</span>
    <span class="info-item-value">{vendorCount}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Games detected</span>
    <span class="info-item-value">{Math.round(gameTween.current)}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Backups stored</span>
    <span class="info-item-value">{Math.round(backupTween.current)}</span>
  </div>
  <div class="info-item">
    <span class="info-item-label">Manifest updated</span>
    <span class="info-item-value is-mono">{$manifestUpdatedAt || "—"}</span>
  </div>
</section>

<div class="info-grid" in:fade={{ duration: 240, delay: 120 }}>
  <section class="surface">
    <header class="surface-head">
      <Database size={16} />
      <h3 class="surface-title">Manifest</h3>
      <span class="status-pill is-{$catalogStatus.kind}">
        <span class="status-dot"></span>
        {$catalogStatus.label}
      </span>
    </header>
    <p class="surface-sub">Live mirror of every upstream upscaler vendor. Status reflects the last refresh attempt.</p>
    <dl class="meta-grid">
      <dt>Last update</dt><dd class="mono">{$manifestUpdatedAt || "—"}</dd>
      <dt>Total versions</dt><dd class="mono">{releaseCount.toLocaleString()}</dd>
      <dt>Vendors tracked</dt><dd>{vendorCount}</dd>
      <dt>DLL families</dt><dd>{familyCount}</dd>
    </dl>
  </section>

  <section class="surface">
    <header class="surface-head">
      <FolderOpen size={16} />
      <h3 class="surface-title">Data &amp; logs</h3>
    </header>
    <p class="surface-sub">Everything DLSSync writes is plain JSON or SQLite under your user profile. Nothing leaves your machine.</p>
    <ul class="path-list">
      <li class="path-row">
        <span class="path-label">Root</span>
        <span class="path-value mono truncate" title={appPaths?.root ?? ""}>{appPaths?.root ?? "—"}</span>
        <button class="path-btn" onclick={openRoot} disabled={!appPaths} aria-label="Open root folder"><ExternalLink size={11} /></button>
      </li>
      <li class="path-row">
        <span class="path-label">Settings</span>
        <span class="path-value mono truncate" title={appPaths?.settings_file ?? ""}>{appPaths?.settings_file ?? "—"}</span>
        <button class="path-btn" onclick={revealConfig} disabled={!appPaths} aria-label="Reveal settings file"><FileText size={11} /></button>
      </li>
      <li class="path-row">
        <span class="path-label">Backups</span>
        <span class="path-value mono truncate" title={appPaths?.backups_dir ?? ""}>{appPaths?.backups_dir ?? "—"}</span>
        <button class="path-btn" onclick={openBackups} disabled={!appPaths} aria-label="Open backups folder"><ExternalLink size={11} /></button>
      </li>
      <li class="path-row">
        <span class="path-label">Logs</span>
        <span class="path-value mono truncate" title={appPaths?.logs_dir ?? ""}>{appPaths?.logs_dir ?? "—"}</span>
        <button class="path-btn" onclick={openLogs} disabled={!appPaths} aria-label="Open logs folder"><ExternalLink size={11} /></button>
      </li>
    </ul>
  </section>
</div>

<section class="surface system-surface" in:fade={{ duration: 260, delay: 180 }}>
  <header class="surface-head">
    <Cpu size={16} />
    <h3 class="surface-title">Your system</h3>
    <span class="surface-sub-inline">Detected once per launch. Never sent off your machine.</span>
  </header>
  {#if systemInfoFailed}
    <p class="system-empty">System detection unavailable in this build.</p>
  {:else if !systemInfo}
    <div class="system-skel">
      {#each Array(3) as _}
        <div class="system-skel-row">
          <span class="skeleton skel-label"></span>
          <span class="skeleton skel-value"></span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="system-grid">
      <div class="system-row">
        <span class="system-label"><Monitor size={13} /> OS</span>
        <span class="system-value">
          {systemInfo.os.edition || systemInfo.os.name}
          {#if systemInfo.os.version}<span class="system-muted mono">{systemInfo.os.version}</span>{/if}
          {#if systemInfo.os.build}<span class="system-muted mono">build {systemInfo.os.build}</span>{/if}
        </span>
      </div>
      <div class="system-row">
        <span class="system-label"><Cpu size={13} /> CPU</span>
        <span class="system-value">
          {systemInfo.cpu.brand}
          <span class="system-muted">·</span>
          <span class="system-muted">{systemInfo.cpu.physical_cores} cores / {systemInfo.cpu.logical_cores} threads</span>
        </span>
      </div>
      <div class="system-row">
        <span class="system-label"><MemoryStick size={13} /> RAM</span>
        <span class="system-value">
          {fmtBytes(systemInfo.ram.total_bytes)}
          {#if systemInfo.ram.modules.length > 0}
            {@const types = Array.from(new Set(systemInfo.ram.modules.map((m) => m.type_label).filter((t) => t && t !== "Unknown")))}
            {@const speeds = Array.from(new Set(systemInfo.ram.modules.map((m) => m.mhz).filter((s) => s > 0)))}
            {#if types.length > 0}<span class="system-muted">·</span><span class="system-muted">{types.join(" / ")}</span>{/if}
            {#if speeds.length > 0}<span class="system-muted">·</span><span class="system-muted">{speeds.join(" / ")} MHz</span>{/if}
            <span class="system-muted">·</span>
            <span class="system-muted">{systemInfo.ram.modules.length} module{systemInfo.ram.modules.length === 1 ? "" : "s"}</span>
          {/if}
        </span>
      </div>
      {#each systemInfo.gpus as gpu, idx}
        <div class="system-row">
          <span class="system-label"><HardDrive size={13} /> {systemInfo.gpus.length > 1 ? `GPU ${idx + 1}` : "GPU"}</span>
          <span class="system-value">
            <span class="chip chip-neutral">
              {#if gpu.vendor === "other"}
                {vendorLabel(gpu.vendor)}
              {:else}
                <BrandMark key={gpu.vendor} tone="mono" size={12} />
              {/if}
            </span>
            {gpu.model}
            {#if gpu.driver_version && gpu.driver_version !== "Unknown"}
              <span class="system-muted mono">driver {gpu.driver_version}</span>
            {/if}
            {#if gpu.vram_bytes > 0}
              <span class="system-muted">·</span>
              <span class="system-muted">{fmtBytes(gpu.vram_bytes)} VRAM</span>
            {/if}
            {#if gpu.recommended_runtimes.length > 0}
              <span class="system-muted">·</span>
              {#each gpu.recommended_runtimes as runtime}
                <span class="chip chip-accent rec-chip">{runtime}</span>
              {/each}
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</section>

<section class="sources-section" in:fade={{ duration: 240, delay: 220 }}>
  <header class="section-head-row">
    <h3 class="section-heading-h">Manifest sources</h3>
    <p class="section-sub">Versions, dates, hashes and signature data come exclusively from these upstream projects.</p>
  </header>
  <div class="source-grid">
    {#each SOURCES as s, i}
      <button
        class="source-card"
        style:--src-accent={s.accent}
        onclick={() => openExternal(s.url)}
        in:fly={{ y: 4, duration: 220, delay: 240 + i * 30 }}
      >
        <span class="source-stripe"></span>
        <span class="source-vendor"><BrandMark key={s.vendor} tone="mono" size={12} /></span>
        <span class="source-label">{s.label}</span>
        <ExternalLink size={11} />
      </button>
    {/each}
  </div>
</section>

<section class="author-section" in:fade={{ duration: 240, delay: 280 }}>
  <header class="section-head-row">
    <h3 class="section-heading-h">Made by</h3>
    <p class="section-sub">DLSSync is an independent open-source project under the Apache 2.0 license.</p>
  </header>
  <div class="author-card">
    <div class="author-identity">
      <div class="author-avatar" aria-hidden="true">
        <span class="avatar-glyph mono">x.</span>
      </div>
      <div class="author-meta">
        <span class="author-handle">xt0n1</span>
        <span class="author-role">Author &amp; maintainer</span>
      </div>
    </div>
    <div class="author-links">
      <button class="author-link" onclick={() => openExternal("https://github.com/xt0n1-t3ch")} title="GitHub @xt0n1-t3ch">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 .3a12 12 0 0 0-3.79 23.4c.6.11.82-.26.82-.58v-2c-3.34.73-4.04-1.61-4.04-1.61-.55-1.4-1.34-1.77-1.34-1.77-1.1-.75.08-.73.08-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.84 2.81 1.31 3.5 1 .1-.78.42-1.31.76-1.61-2.66-.3-5.46-1.33-5.46-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.12-3.17 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.65.25 2.87.12 3.17.77.84 1.24 1.91 1.24 3.22 0 4.61-2.8 5.62-5.47 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58A12 12 0 0 0 12 .3Z"/></svg>
        <span class="author-link-text">github.com/xt0n1-t3ch</span>
      </button>
      <button class="author-link" onclick={() => openExternal("https://discord.com/users/211189703641268224")} title="Discord">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M20.317 4.369A19.79 19.79 0 0 0 16.558 3.2a.074.074 0 0 0-.079.038c-.342.61-.72 1.405-.987 2.027a18.27 18.27 0 0 0-5.484 0 12.65 12.65 0 0 0-1.001-2.027.077.077 0 0 0-.078-.038A19.736 19.736 0 0 0 5.171 4.369a.07.07 0 0 0-.032.027C1.533 9.79.583 14.95 1.05 20.04a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.029.077.077 0 0 0 .084-.027 14.24 14.24 0 0 0 1.226-1.994.075.075 0 0 0-.041-.104 13.1 13.1 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .078-.011c3.927 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .079.01c.12.1.246.198.373.292a.077.077 0 0 1-.006.128c-.598.349-1.22.645-1.873.892a.077.077 0 0 0-.041.105c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .031-.055c.5-5.876-.838-10.998-3.548-15.644a.06.06 0 0 0-.031-.028zM8.02 16.937c-1.182 0-2.157-1.085-2.157-2.418 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.419 0 1.333-.956 2.418-2.157 2.418zm7.974 0c-1.182 0-2.157-1.085-2.157-2.418 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.419 0 1.333-.946 2.418-2.157 2.418z"/></svg>
        <span class="author-link-text">Discord</span>
      </button>
      <button class="author-link" onclick={() => openExternal("https://xt0n1.com")} title="xt0n1.com">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
        <span class="author-link-text">xt0n1.com</span>
      </button>
    </div>
  </div>
</section>

<section class="support-section" in:fade={{ duration: 240, delay: 300 }}>
  <header class="section-head-row">
    <h3 class="section-heading-h">Help DLSSync grow</h3>
    <p class="section-sub">DLSSync is free, with zero telemetry and no paid tier. The fastest way to support it is a star, an endorsement, or a share — each one helps other gamers find it.</p>
  </header>
  <div class="support-grid">
    <button class="support-cta is-star" onclick={() => openExternal(EXTERNAL_URLS.homepage)} title="Star DLSSync on GitHub">
      <Star size={16} fill="currentColor" />
      <span class="support-cta-label">Star on GitHub</span>
      {#if starCount !== null}<span class="support-cta-count mono">{starCount.toLocaleString()}</span>{/if}
    </button>
    <button class="support-cta is-endorse" onclick={() => openExternal(EXTERNAL_URLS.nexusMod)} title="Endorse DLSSync on Nexus Mods">
      <NexusLogo size={18} />
      <span class="support-cta-label">Endorse on Nexus</span>
    </button>
    <button class="support-cta" onclick={shareApp} title="Copy a shareable link">
      <Share2 size={16} />
      <span class="support-cta-label">Share with a friend</span>
    </button>
  </div>
</section>

<footer class="about-foot" in:fade={{ duration: 220, delay: 340 }}>
  <p>{TRADEMARKS_LINE}</p>
  <p class="foot-disclaimer">Not endorsed by, sponsored by, or affiliated with NVIDIA, Intel, AMD, or Microsoft. DLSSync is an independent project — every redistributed binary retains its original vendor Authenticode signature.</p>
  <p class="foot-license">Licensed under <button class="foot-link" onclick={() => openExternal("https://www.apache.org/licenses/LICENSE-2.0")}>Apache 2.0</button> · Copyright 2026 xt0n1</p>
</footer>

<style>
  .sponsor-btn svg { color: #db61a2; }
  .sponsor-btn:hover svg { color: #f0707f; }
  .kofi-btn svg { color: #ff5e5b; }
  .kofi-btn:hover svg { color: #ff7674; }
  .brand-row {
    display: grid;
    grid-template-columns: 56px 1fr auto;
    gap: 18px;
    align-items: center;
    padding: 16px 22px;
    margin-bottom: 18px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    position: relative;
    overflow: hidden;
  }
  .brand-row::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 1px;
    background: var(--border);
  }
  .brand-mark {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-md);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-dim);
    color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent-ring);
  }
  .brand-text { min-width: 0; }
  .brand-title-row { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
  .brand-title {
    font-size: var(--fs-xl);
    font-weight: 700;
    letter-spacing: var(--letter-tighter);
    color: var(--text-primary);
    line-height: 1.1;
  }
  .whats-new {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px 20px;
    margin-bottom: 22px;
  }
  .wn-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .wn-eyebrow {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
  }
  .wn-version {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .wn-link {
    margin-left: auto;
    background: transparent;
    border: none;
    color: var(--accent);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: background var(--dur-fast) var(--ease);
  }
  .wn-link:hover { background: var(--accent-dim); }
  .wn-link:focus-visible { outline: none; box-shadow: var(--shadow-ring); }
  .wn-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .wn-list li {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    padding-left: 18px;
    position: relative;
    line-height: var(--lh-snug);
  }
  .wn-list li::before {
    content: '';
    position: absolute;
    left: 4px;
    top: 8px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
  }

  .brand-version {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-dim);
    padding: 2px 9px;
    border-radius: var(--radius-full);
    letter-spacing: 0;
  }
  .brand-tagline {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .brand-pillars { display: flex; gap: 10px; flex-wrap: wrap; }
  .pillar {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    font-weight: 500;
    padding: 6px 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: help;
  }
  .pillar :global(svg) { color: var(--accent); flex-shrink: 0; }

  @media (max-width: 760px) {
    .brand-row { grid-template-columns: 56px 1fr; }
    .brand-pillars { grid-column: 1 / -1; }
  }

  .update-banner {
    padding: 10px 16px;
    margin-bottom: 18px;
    border-radius: var(--radius-md);
    font-size: var(--fs-sm);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-secondary);
  }
  .update-banner.is-success { color: var(--success); border-color: color-mix(in oklab, var(--success) 40%, transparent); background: var(--success-dim); }
  .update-banner.is-warning { color: var(--warning); border-color: color-mix(in oklab, var(--warning) 40%, transparent); background: var(--warning-dim); }
  .update-banner.is-danger { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 40%, transparent); background: var(--danger-dim); }
  .update-banner.is-info { color: var(--accent); border-color: var(--accent-ring); background: var(--accent-soft); }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .surface {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 18px 20px;
    transition: border-color var(--dur-fast) var(--ease);
  }
  .surface:hover { border-color: var(--border-hover); }
  .surface-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .surface-head :global(svg) { color: var(--accent); flex-shrink: 0; }
  .surface-title {
    font-size: var(--fs-md);
    font-weight: 600;
    letter-spacing: var(--letter-tight);
    color: var(--text-primary);
    margin: 0;
  }
  .surface-sub {
    font-size: var(--fs-sm);
    color: var(--text-secondary);
    line-height: var(--lh-snug);
    margin-bottom: 14px;
  }
  .surface-sub-inline {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    margin-left: auto;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-2xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    padding: 3px 9px;
    border-radius: var(--radius-full);
    margin-left: auto;
  }
  .status-pill.is-accent { color: var(--accent); background: var(--accent-dim); }
  .status-pill.is-success { color: var(--success); background: var(--success-dim); }
  .status-pill.is-warning { color: var(--warning); background: var(--warning-dim); }
  .status-pill.is-danger { color: var(--danger); background: var(--danger-dim); }
  .status-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; box-shadow: 0 0 6px currentColor; }

  .meta-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px 18px;
    margin: 0;
  }
  .meta-grid dt {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .meta-grid dd {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    margin: 0;
    text-align: right;
  }

  .path-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .path-row {
    display: grid;
    grid-template-columns: 76px 1fr 24px;
    gap: 10px;
    align-items: center;
    padding: 7px 10px;
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
  }
  .path-label {
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    color: var(--text-muted);
    font-weight: 600;
  }
  .path-value { font-size: var(--fs-xs); color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path-btn {
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    background: transparent;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .path-btn:hover:not(:disabled) { color: var(--accent); background: var(--accent-dim); }
  .path-btn:disabled { opacity: 0.3; cursor: not-allowed; }

  .system-surface { margin-bottom: 16px; }
  .system-grid { display: flex; flex-direction: column; gap: 8px; }
  .system-row {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 14px;
    align-items: baseline;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
  }
  .system-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 700;
    color: var(--text-muted);
  }
  .system-label :global(svg) { color: var(--accent); }
  .system-value {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    display: inline-flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .system-muted { color: var(--text-muted); font-size: var(--fs-xs); }
  .rec-chip { font-size: var(--fs-2xs); padding: 2px 7px; }
  .system-empty {
    padding: 18px 4px;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    font-style: italic;
  }
  .system-skel { display: flex; flex-direction: column; gap: 8px; }
  .system-skel-row { display: grid; grid-template-columns: 110px 1fr; gap: 14px; padding: 10px 14px; background: var(--bg-elevated); border-radius: var(--radius-md); }
  .skel-label { height: 11px; width: 60%; }
  .skel-value { height: 11px; width: 80%; }

  .sources-section { margin-bottom: 16px; }
  .section-head-row { margin-bottom: 12px; }
  .section-heading-h {
    font-size: var(--fs-lg);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--letter-wide);
    color: var(--text-primary);
    margin-bottom: 3px;
  }
  .source-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 8px;
  }
  .source-card {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    gap: 0 8px;
    padding: 10px 14px 10px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
    overflow: hidden;
  }
  .source-card:hover {
    border-color: var(--src-accent, var(--accent));
    background: var(--bg-card-hover);
    transform: translateY(-1px);
  }
  .source-stripe {
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 3px;
    background: var(--src-accent, var(--accent));
    opacity: 0.7;
  }
  .source-vendor {
    grid-column: 1;
    grid-row: 1;
    font-size: var(--fs-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 700;
    color: var(--src-accent, var(--text-muted));
  }
  .source-label {
    grid-column: 1;
    grid-row: 2;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
  }
  .source-card :global(svg) {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    color: var(--text-muted);
    opacity: 0.5;
    transition: opacity var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .source-card:hover :global(svg) { opacity: 1; color: var(--src-accent, var(--accent)); }

  .author-section { margin-bottom: 14px; }
  .author-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    flex-wrap: wrap;
    position: relative;
    overflow: hidden;
  }
  .author-card::before {
    content: '';
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 2px;
    background: var(--accent);
  }
  .author-identity {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .author-avatar {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: inset 0 0 0 1px var(--accent-ring);
  }
  .avatar-glyph {
    font-size: var(--fs-md);
    font-weight: 700;
    letter-spacing: -0.04em;
  }
  .author-meta { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .author-handle {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .author-role {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-wider);
    font-weight: 600;
  }
  .author-links {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .author-link {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .author-link:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
    border-color: var(--accent-ring);
    transform: translateY(-1px);
  }
  .author-link :global(svg) { color: var(--text-muted); transition: color var(--dur-fast) var(--ease); flex-shrink: 0; }
  .author-link:hover :global(svg) { color: var(--accent); }

  .about-foot {
    margin-top: 22px;
    padding: 14px 18px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: var(--fs-xs);
    color: var(--text-muted);
    line-height: var(--lh-normal);
  }
  .about-foot p + p { margin-top: 8px; }
  .foot-disclaimer { font-size: var(--fs-2xs); opacity: 0.85; }
  .foot-license {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
    padding-top: 8px;
    border-top: 1px solid var(--border);
    margin-top: 10px !important;
  }
  .foot-link {
    background: none;
    border: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
    padding: 0;
  }
  .foot-link:hover { text-decoration: underline; }

  .support-section { margin-bottom: 14px; }
  .support-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
  }
  .support-cta {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    color: var(--text-secondary);
    transition: color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .support-cta:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
    transform: translateY(-2px);
  }
  .support-cta.is-star :global(svg) { color: var(--gh-star); }
  .support-cta.is-star:hover { border-color: var(--gh-star); }
  .support-cta.is-endorse:hover { border-color: var(--nexus); }
  .support-cta :global(svg) { color: var(--text-muted); flex-shrink: 0; transition: color var(--dur-fast) var(--ease); }
  .support-cta-label { font-size: var(--fs-sm); font-weight: 600; }
  .support-cta-count {
    margin-left: auto;
    font-size: var(--fs-xs);
    color: var(--accent);
    background: var(--accent-dim);
    padding: 2px 8px;
    border-radius: var(--radius-full);
    font-variant-numeric: tabular-nums;
  }

  .spin { width: 12px; height: 12px; border: 2px solid currentColor; border-top-color: transparent; border-radius: 50%; animation: spin 0.7s linear infinite; display: inline-block; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
