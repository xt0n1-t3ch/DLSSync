<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invokeCommand as transport, COMMANDS } from "../generated/bindings";
  import { get } from "svelte/store";
  import { fly, slide } from "svelte/transition";
  import { showToast, updateBannerActive } from "../lib/stores";
  import {
    notifications,
    pushNotification,
    makeNotificationEntry,
  } from "../lib/notifications";
  import { githubReleaseTagUrl } from "../lib/ux";
  import { t, locale, translate } from "../lib/i18n/index";
  import Download from "@lucide/svelte/icons/download";
  import X from "@lucide/svelte/icons/x";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import { appUpdaterEnabled } from "../lib/distribution";

  type UpdateInfo = {
    version: string;
    notes: string | null;
  };

  type RuntimeMode = {
    portable: boolean;
    release_url: string;
  };

  type Stage = "idle" | "available" | "downloading" | "installing" | "error";

  let stage = $state<Stage>("idle");
  let available = $state<UpdateInfo | null>(null);
  let runtime = $state<RuntimeMode | null>(null);
  let downloadProgress = $state(0);
  let errorMessage = $state("");
  let dismissedVersion = $state<string | null>(null);
  let changelogOpen = $state(false);
  let timer: ReturnType<typeof setInterval> | null = null;

  const POLL_INTERVAL_MS = 6 * 60 * 60 * 1000;
  const DISMISS_KEY = "dlssync-update-dismissed";

  onMount(() => {
    if (!appUpdaterEnabled) return;
    try {
      dismissedVersion = localStorage.getItem(DISMISS_KEY);
    } catch {
      dismissedVersion = null;
    }
    void startUpdaterChecks();
  });

  async function startUpdaterChecks(): Promise<void> {
    await loadRuntimeMode();
    if (runtime?.portable) return;
    await checkForUpdates();
    timer = setInterval(() => { void checkForUpdates(); }, POLL_INTERVAL_MS);
    window.addEventListener("dlssync:check-updates", handleExternalCheck);
  }

  async function loadRuntimeMode(): Promise<void> {
    try {
      runtime = await transport<RuntimeMode>(COMMANDS.runtime_mode);
    } catch {
      runtime = null;
    }
  }

  function handleExternalCheck(e: Event): void {
    if (!appUpdaterEnabled || runtime?.portable) return;
    const detail = (e as CustomEvent<{ force?: boolean }>).detail;
    if (detail?.force) {
      try { localStorage.removeItem(DISMISS_KEY); } catch {}
      dismissedVersion = null;
    }
    void checkForUpdates();
  }

  onDestroy(() => {
    if (timer) clearInterval(timer);
    window.removeEventListener("dlssync:check-updates", handleExternalCheck);
    updateBannerActive.set(false);
  });

  $effect(() => {
    updateBannerActive.set(stage !== "idle" && available !== null);
  });

  function devFakeUpdate(): UpdateInfo | null {
    const params = new URLSearchParams(window.location.search);
    const fake = params.get("fakeUpdate");
    if (!fake) return null;
    return {
      version: fake.replace(/^v/, ""),
      notes:
        "### Fixed\n\n" +
        "- Updating Intel XeSS no longer fails halfway through with random network errors. The app now downloads the shared archive only once instead of four times.\n" +
        "- Slow connections no longer time out mid-download. The timeout is per chunk, not per request.\n" +
        "- Transient GitHub CDN flakes (TCP reset, 503, 429) now retry automatically with backoff.\n\n" +
        "### Apply progress modal — rebuilt\n\n" +
        "- Failure-centric layout with filter chips and per-stage timeline.\n" +
        "- One-click `Retry all failed` and `Allow unsigned & retry` actions.\n" +
        "- `Copy report` dumps a full text report for bug reports.\n\n" +
        "### Added\n\n" +
        "- Tray badge with in-flight apply count.\n" +
        "- Configurable apply concurrency in Settings → Advanced.\n" +
        "- Network section with retry, timeout, and cache TTL knobs.",
    };
  }

  function firstChangelogHeading(notes: string | null): string | null {
    if (!notes) return null;
    for (const raw of notes.split("\n")) {
      const line = raw.trim();
      if (line.startsWith("#")) return line.replace(/^#+\s*/, "");
    }
    return null;
  }

  const NOTIFICATION_BODY_MAX = 160;

  function notificationBody(notes: string | null): string {
    const items = notes ? renderNotes(notes).flatMap((section) => section.items) : [];
    if (items.length === 0) {
      return firstChangelogHeading(notes) ?? translate(get(locale), "component.banner.notifBodyFallback");
    }
    const summary = items.slice(0, 2).join(" · ");
    return summary.length > NOTIFICATION_BODY_MAX
      ? `${summary.slice(0, NOTIFICATION_BODY_MAX - 1)}…`
      : summary;
  }

  function emitAppUpdateNotification(next: UpdateInfo): void {
    const existing = get(notifications).find(
      (n) => n.kind === "app_update_available" && (n.title.includes(next.version) || n.body?.includes(next.version)),
    );
    if (existing) return;
    const entry = makeNotificationEntry(
      "app_update_available",
      translate(get(locale), "component.banner.notifTitle", { version: next.version }),
      notificationBody(next.notes),
      { link: githubReleaseTagUrl(next.version) },
    );
    pushNotification(entry).catch((err) => console.warn("[dlssync] push app-update notification failed:", err));
  }

  async function checkForUpdates(): Promise<void> {
    if (!appUpdaterEnabled || runtime?.portable) return;
    if (stage === "downloading" || stage === "installing") return;
    const fake = devFakeUpdate();
    if (fake) {
      if (dismissedVersion === fake.version) return;
      available = fake;
      stage = "available";
      emitAppUpdateNotification(fake);
      return;
    }
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (!update || (update as { available?: boolean }).available === false) {
        return;
      }
      const next: UpdateInfo = {
        version: (update as { version?: string }).version ?? "unknown",
        notes: (update as { body?: string }).body ?? null,
      };
      if (dismissedVersion === next.version) return;
      available = next;
      stage = "available";
      emitAppUpdateNotification(next);
    } catch {
      // updater endpoint unreachable or no release published yet — silent retry on next poll
    }
  }

  async function openReleasePage(): Promise<void> {
    if (!runtime) return;
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(runtime.release_url);
      stage = "idle";
      available = null;
      changelogOpen = false;
    } catch (err) {
      stage = "error";
      errorMessage = String(err);
    }
  }

  async function devFakeApply(): Promise<void> {
    stage = "downloading";
    downloadProgress = 0;
    for (let i = 0; i <= 100; i += 5) {
      await new Promise((r) => setTimeout(r, 60));
      downloadProgress = i;
    }
    stage = "installing";
    await new Promise((r) => setTimeout(r, 800));
    showToast("success", "[DEV] Mock update flow finished — would restart now");
    stage = "idle";
    available = null;
    changelogOpen = false;
  }

  async function applyUpdate(): Promise<void> {
    if (!appUpdaterEnabled) return;
    if (!available || stage === "downloading" || stage === "installing") return;
    if (runtime?.portable) {
      await openReleasePage();
      return;
    }
    if (devFakeUpdate()) {
      await devFakeApply();
      return;
    }
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (!update) {
        stage = "error";
        errorMessage = translate(get(locale), "component.banner.noLongerAvailable");
        return;
      }
      stage = "downloading";
      downloadProgress = 0;
      let downloaded = 0;
      let contentLength = 0;
      await (update as {
        downloadAndInstall: (cb: (e: { event: string; data?: { contentLength?: number; chunkLength?: number } }) => void) => Promise<void>;
      }).downloadAndInstall((event) => {
        if (event.event === "Started" && event.data?.contentLength) {
          contentLength = event.data.contentLength;
        } else if (event.event === "Progress" && event.data?.chunkLength) {
          downloaded += event.data.chunkLength;
          if (contentLength > 0) {
            downloadProgress = Math.min(100, Math.round((downloaded / contentLength) * 100));
          }
        } else if (event.event === "Finished") {
          stage = "installing";
        }
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      showToast("success", translate(get(locale), "component.banner.toastInstalled"));
      await relaunch();
    } catch (err: unknown) {
      stage = "error";
      errorMessage = String(err);
      showToast("danger", translate(get(locale), "component.banner.toastFailed", { error: errorMessage }));
    }
  }

  function skip(): void {
    if (!available) return;
    try { localStorage.setItem(DISMISS_KEY, available.version); } catch {}
    dismissedVersion = available.version;
    stage = "idle";
    available = null;
    changelogOpen = false;
  }

  function later(): void {
    stage = "idle";
    available = null;
    changelogOpen = false;
  }

  function toggleChangelog(): void {
    changelogOpen = !changelogOpen;
  }

  function renderNotes(notes: string): { heading: string | null; items: string[] }[] {
    const sections: { heading: string | null; items: string[] }[] = [];
    let current: { heading: string | null; items: string[] } = { heading: null, items: [] };
    const linkRef = /^\[[^\]]+\]:\s/;
    for (const raw of notes.split(/\r?\n/)) {
      const line = raw.trim();
      if (!line) continue;
      if (linkRef.test(line)) continue;
      const headingMatch = line.match(/^#{1,6}\s+(.+)$/);
      if (headingMatch) {
        if (current.heading !== null || current.items.length > 0) sections.push(current);
        current = { heading: headingMatch[1].trim(), items: [] };
        continue;
      }
      if (line.startsWith("- ") || line.startsWith("* ")) {
        current.items.push(line.slice(2).trim());
      } else {
        current.items.push(line);
      }
    }
    if (current.heading !== null || current.items.length > 0) sections.push(current);
    return sections;
  }
</script>

{#if appUpdaterEnabled && stage !== "idle" && available}
  <div
    class="update-banner"
    class:downloading={stage === "downloading"}
    class:installing={stage === "installing"}
    class:error={stage === "error"}
    role="alert"
    aria-live="polite"
    in:fly={{ y: 16, duration: 280, opacity: 0 }}
    out:fly={{ y: 16, duration: 200, opacity: 0 }}
  >
    <div class="banner-head">
      <span class="banner-mark" aria-hidden="true">
        <Download size={16} strokeWidth={2.2} />
      </span>
      <div class="banner-body">
        {#if stage === "available"}
          <span class="banner-title">{$t("component.banner.available", { version: available.version })}</span>
          {#if runtime?.portable}
            <span class="banner-sub">{$t("component.banner.subPortable")}</span>
          {:else}
            <span class="banner-sub">{$t("component.banner.subInstaller")}</span>
          {/if}
        {:else if stage === "downloading"}
          <span class="banner-title">{$t("component.banner.downloading", { version: available.version })}</span>
          <span class="banner-sub mono">{downloadProgress}%</span>
        {:else if stage === "installing"}
          <span class="banner-title">{$t("component.banner.installing", { version: available.version })}</span>
          <span class="banner-sub">{$t("component.banner.subRestarting")}</span>
        {:else if stage === "error"}
          <span class="banner-title">{$t("component.banner.failedTitle")}</span>
          <span class="banner-sub">{errorMessage}</span>
        {/if}
      </div>
      {#if stage === "available" || stage === "error"}
        <button class="banner-close" onclick={later} aria-label={$t("common.dismiss")}><X size={14} strokeWidth={2.2} /></button>
      {/if}
    </div>

    {#if stage === "downloading"}
      <div class="banner-progress" aria-hidden="true">
        <div class="banner-progress-fill" style:width="{downloadProgress}%"></div>
      </div>
    {/if}

    {#if stage === "available" && available.notes}
      <button
        class="changelog-toggle"
        onclick={toggleChangelog}
        aria-expanded={changelogOpen}
        type="button"
      >
        <ChevronDown size={12} strokeWidth={2.4} class="changelog-chev {changelogOpen ? 'open' : ''}" />
        <span>{changelogOpen ? $t("component.banner.hideChangelog") : $t("component.banner.viewChangelog")}</span>
      </button>

      {#if changelogOpen}
        <div class="changelog-body" transition:slide={{ duration: 200 }}>
          {#each renderNotes(available.notes) as section}
            {#if section.heading}
              <div class="changelog-heading">{section.heading}</div>
            {/if}
            {#if section.items.length > 0}
              <ul class="changelog-list">
                {#each section.items as item}
                  <li>{item}</li>
                {/each}
              </ul>
            {/if}
          {/each}
        </div>
      {/if}
    {/if}

    {#if stage === "available"}
      <div class="banner-actions">
        <button class="banner-btn banner-btn-ghost" onclick={later} type="button">
          {$t("component.banner.later")}
        </button>
        <button class="banner-btn banner-btn-ghost" onclick={skip} type="button">
          {$t("component.banner.skip")}
        </button>
        <button class="banner-btn banner-btn-primary" onclick={applyUpdate} type="button">
          {#if runtime?.portable}
            <ExternalLink size={13} strokeWidth={2.2} />
            {$t("component.banner.openRelease")}
          {:else}
            <Download size={13} strokeWidth={2.2} />
            {$t("component.banner.updateNow")}
          {/if}
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    position: fixed;
    bottom: 18px;
    left: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px 14px 18px;
    width: min(440px, calc(100vw - 36px));
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    z-index: 200;
  }
  .update-banner.downloading,
  .update-banner.installing {
    border-color: var(--accent-ring);
  }
  .update-banner.error {
    border-color: rgba(239, 68, 68, 0.5);
  }

  .banner-head {
    display: grid;
    grid-template-columns: 28px 1fr auto;
    gap: 12px;
    align-items: start;
  }
  .banner-mark {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background: var(--accent-dim);
    color: var(--accent);
    flex-shrink: 0;
  }
  .update-banner.error .banner-mark {
    background: var(--danger-dim);
    color: var(--danger);
  }
  .banner-body {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    padding-top: 1px;
  }
  .banner-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .banner-sub {
    font-size: var(--fs-xs);
    color: var(--text-muted);
    line-height: var(--lh-snug);
  }
  .banner-close {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    flex-shrink: 0;
  }
  .banner-close:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }

  .banner-progress {
    height: 3px;
    border-radius: var(--radius-full);
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .banner-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width var(--dur-normal) var(--ease-out);
  }

  .changelog-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    padding: 4px 0;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: color var(--dur-fast) var(--ease);
  }
  .changelog-toggle:hover {
    color: var(--text-primary);
  }
  .changelog-toggle :global(.changelog-chev) {
    transition: transform var(--dur-normal) var(--ease-out);
  }
  .changelog-toggle :global(.changelog-chev.open) {
    transform: rotate(180deg);
  }
  .changelog-body {
    max-height: 200px;
    overflow-y: auto;
    padding: 4px 4px 4px 0;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
  .changelog-heading {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 8px;
    margin-bottom: 4px;
    letter-spacing: var(--letter-tight);
  }
  .changelog-heading:first-child {
    margin-top: 0;
  }
  .changelog-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .changelog-list li {
    position: relative;
    padding-left: 12px;
    font-size: var(--fs-xs);
    color: var(--text-muted);
    line-height: var(--lh-snug);
  }
  .changelog-list li::before {
    content: "";
    position: absolute;
    left: 0;
    top: 7px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .banner-actions {
    display: flex;
    gap: 6px;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
  }
  .banner-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    font-size: var(--fs-xs);
    font-weight: 600;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
    white-space: nowrap;
  }
  .banner-btn-ghost {
    background: transparent;
    color: var(--text-muted);
  }
  .banner-btn-ghost:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .banner-btn-primary {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
  }
  .banner-btn-primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  @media (max-width: 560px) {
    .update-banner {
      width: calc(100vw - 24px);
      left: 12px;
      right: 12px;
      bottom: 12px;
    }
  }
</style>
