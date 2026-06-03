<script lang="ts">
  import { onMount } from "svelte";
  import {
    notifications,
    markRead,
    markAllRead,
    dismiss,
    refreshNotifications,
    vendorKeyForNotification,
    type NotificationEntry,
    type NotificationKind,
  } from "../lib/notifications";
  import { currentView } from "../lib/stores";
  import { formatDurationSecs } from "../lib/formatHuman";
  import { EXTERNAL_URLS } from "../lib/ux";
  import { t, translate, locale } from "../lib/i18n/index";
  import { get } from "svelte/store";
  import BrandMark from "./BrandMark.svelte";

  let { open, onClose }: { open: boolean; onClose: () => void } = $props();
  let panelEl: HTMLDivElement | undefined = $state();

  onMount(() => {
    void refreshNotifications();
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape" && open) {
        e.preventDefault();
        onClose();
      }
    };
    const onClickOutside = (e: MouseEvent): void => {
      if (!open) return;
      const target = e.target as Node | null;
      if (target instanceof Element && target.closest("[data-notifications-toggle]")) return;
      if (panelEl && target && !panelEl.contains(target)) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onClickOutside);
    return () => {
      window.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onClickOutside);
    };
  });

  let entries = $derived($notifications);

  async function handleItemClick(entry: NotificationEntry): Promise<void> {
    if (entry.read_at == null) {
      try {
        await markRead(entry.id);
      } catch (err) {
        console.warn("[dlssync] mark read failed:", err);
      }
    }
    if (entry.kind === "app_update_available") {
      window.dispatchEvent(new CustomEvent("dlssync:check-updates", { detail: { force: true } }));
      onClose();
    } else if (entry.kind === "catalog_update_available") {
      currentView.set("catalog");
      onClose();
    } else if (entry.kind === "driver_update_available" || entry.kind === "system_driver_update_available") {
      currentView.set("drivers");
      onClose();
    } else if (entry.kind === "dll_updates_available") {
      currentView.set("library");
      onClose();
    } else if (entry.kind === "backup_restored") {
      currentView.set("backups");
      onClose();
    }
  }

  function linkActions(entry: NotificationEntry): { label: string; url: string }[] {
    const loc = get(locale);
    const actions: { label: string; url: string }[] = [];
    if (entry.link) {
      const label =
        entry.kind === "app_update_available"
          ? translate(loc, "component.notif.link.githubRelease")
          : entry.kind === "driver_update_available" || entry.kind === "system_driver_update_available"
            ? translate(loc, "component.notif.link.vendorPage")
            : translate(loc, "common.open");
      actions.push({ label, url: entry.link });
    }
    if (entry.kind === "app_update_available") {
      actions.push({ label: translate(loc, "component.notif.link.nexusMods"), url: EXTERNAL_URLS.nexusMod });
    }
    return actions;
  }

  async function openExternal(url: string, ev: Event): Promise<void> {
    ev.stopPropagation();
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch (err) {
      console.warn("[dlssync] open external link failed:", err);
    }
  }

  async function handleDismiss(entry: NotificationEntry, ev: Event): Promise<void> {
    ev.stopPropagation();
    try {
      await dismiss(entry.id);
    } catch (err) {
      console.warn("[dlssync] dismiss failed:", err);
    }
  }

  async function handleMarkAll(): Promise<void> {
    try {
      await markAllRead();
    } catch (err) {
      console.warn("[dlssync] mark-all failed:", err);
    }
  }

  function relativeTime(iso: string): string {
    const loc = get(locale);
    const then = new Date(iso).getTime();
    const secs = Math.max(0, Math.floor((Date.now() - then) / 1000));
    if (secs < 5) return translate(loc, "component.notif.justNow");
    return translate(loc, "component.notif.relativeAgo", { dur: formatDurationSecs(secs) });
  }

  function kindIcon(kind: NotificationKind): string {
    switch (kind) {
      case "apply_success": return "✓";
      case "apply_failure": return "✕";
      case "apply_cancelled": return "↺";
      case "app_update_available": return "↑";
      case "catalog_update_available": return "★";
      case "dll_updates_available": return "⟳";
      case "driver_update_available":
      case "system_driver_update_available": return "⬇";
      case "backup_restored": return "↺";
      case "scan_failed":
      case "catalog_refresh_failed": return "!";
      default: return "•";
    }
  }

  function tintForKind(kind: NotificationKind): string {
    switch (kind) {
      case "apply_success": return "green";
      case "apply_failure": return "red";
      case "apply_cancelled": return "orange";
      case "app_update_available": return "blue";
      case "catalog_update_available": return "purple";
      case "dll_updates_available": return "blue";
      case "driver_update_available": return "green";
      case "system_driver_update_available": return "purple";
      case "backup_restored": return "green";
      case "scan_failed":
      case "catalog_refresh_failed": return "orange";
      default: return "blue";
    }
  }

</script>

{#if open}
  <div class="bell-panel glass-dialog" role="dialog" aria-label={$t("component.notif.title")} bind:this={panelEl}>
    <header class="bell-panel-header">
      <span class="bell-panel-title">{$t("component.notif.title")}</span>
      <span class="bell-panel-count" aria-label={$t("component.notif.entriesCount", { count: entries.length })}>{entries.length}</span>
    </header>
    <div class="bell-panel-list" role="list">
      {#if entries.length === 0}
        <div class="bell-panel-empty">{$t("component.notif.empty")}</div>
      {:else}
        {#each entries as entry (entry.id)}
          {@const vendorKey = vendorKeyForNotification(entry)}
          <div
            class="bell-item"
            class:bell-item-unread={entry.read_at == null}
            role="listitem"
          >
            {#if entry.read_at == null}
              <span class="bell-unread-stripe" aria-hidden="true"></span>
            {/if}
            <div class="bell-item-row">
              <button
                type="button"
                class="bell-item-main"
                onclick={() => handleItemClick(entry)}
                aria-label="{$t('notifKind.' + entry.kind)}: {entry.title}"
              >
                {#if vendorKey}
                  <span class="bell-item-badge bell-item-logo" aria-hidden="true">
                    <BrandMark key={vendorKey} tone="color" size={18} showLabel={false} />
                  </span>
                {:else}
                  <span class="aura-badge bell-item-badge" data-tint={tintForKind(entry.kind)} aria-hidden="true">
                    {kindIcon(entry.kind)}
                  </span>
                {/if}
                <span class="bell-item-text">
                  <span class="bell-item-title">{entry.title}</span>
                  {#if entry.body}
                    <span class="bell-item-body">{entry.body}</span>
                  {/if}
                  <span class="bell-item-time">{relativeTime(entry.created_at)}</span>
                </span>
              </button>
              <button
                type="button"
                class="bell-item-dismiss"
                title={$t("common.dismiss")}
                aria-label={$t("component.notif.dismissAria")}
                onclick={(ev) => handleDismiss(entry, ev)}
              >
                ×
              </button>
            </div>
            {#if linkActions(entry).length > 0}
              <div class="bell-item-actions">
                {#each linkActions(entry) as action}
                  <button
                    type="button"
                    class="bell-item-link"
                    onclick={(ev) => openExternal(action.url, ev)}
                  >
                    {action.label}
                    <span class="bell-item-link-icon" aria-hidden="true">↗</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
    {#if entries.length > 0}
      <footer class="bell-panel-footer">
        <button type="button" class="bell-panel-action" onclick={handleMarkAll}>
          {$t("component.notif.markAllRead")}
        </button>
      </footer>
    {/if}
  </div>
{/if}

<style>
  .bell-panel {
    position: fixed;
    top: calc(var(--topbar-height) + 6px);
    right: 12px;
    width: 380px;
    max-width: calc(100vw - 24px);
    max-height: min(480px, calc(100vh - var(--topbar-height) - 24px));
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-lg);
    z-index: 200;
  }
  .bell-panel.glass-dialog::before { display: none; }
  @media (max-width: 460px) {
    .bell-panel {
      left: 12px;
      width: auto;
    }
  }
  .bell-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  .bell-panel-title {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: var(--letter-tight);
  }
  .bell-panel-count {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }
  .bell-panel-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bell-panel-empty {
    padding: 32px 14px;
    text-align: center;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
  .bell-item {
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-lg);
    transition: background var(--dur-fast) var(--ease);
  }
  .bell-item:hover {
    background: var(--bg-card-hover);
  }
  .bell-item-row {
    display: flex;
    align-items: stretch;
    gap: 2px;
  }
  .bell-unread-stripe {
    position: absolute;
    left: 4px;
    top: 12px;
    bottom: 12px;
    width: 3px;
    border-radius: var(--radius-full);
    background: var(--accent);
    pointer-events: none;
  }
  .bell-item-main {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 8px 12px 14px;
    text-align: left;
    color: var(--text-primary);
    min-width: 0;
    border-radius: var(--radius-lg);
  }
  .bell-item-main:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .bell-item-badge {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 700;
    line-height: 1;
    flex-shrink: 0;
  }
  .bell-item-logo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }
  .bell-item-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .bell-item-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bell-item-body {
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .bell-item-time {
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    margin-top: 2px;
  }
  .bell-item-dismiss {
    width: 28px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-placeholder);
    font-size: 16px;
    line-height: 1;
    border-radius: var(--radius-sm);
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .bell-item-dismiss:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .bell-item-dismiss:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .bell-item-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 10px 10px 58px;
  }
  .bell-item-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-dim);
    border: 1px solid transparent;
    border-radius: var(--radius-full);
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .bell-item-link:hover {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .bell-item-link:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
  .bell-item-link-icon {
    font-size: 10px;
    opacity: 0.8;
  }
  .bell-panel-footer {
    border-top: 1px solid var(--border);
    padding: 10px 14px;
    display: flex;
    justify-content: flex-end;
  }
  .bell-panel-action {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 6px 12px;
    border-radius: var(--radius-full);
    transition: color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .bell-panel-action:hover {
    color: var(--text-primary);
    background: var(--bg-card-hover);
  }
  .bell-panel-action:focus-visible {
    outline: none;
    box-shadow: var(--shadow-ring);
  }
</style>
