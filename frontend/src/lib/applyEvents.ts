import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  APPLY_INFLIGHT_EVENT,
  APPLY_PROGRESS_EVENT,
  DOWNLOAD_PROGRESS_EVENT,
  TRAY_CHECK_UPDATE_EVENT,
  type ApplyProgress,
  type GroupDownloadProgress,
  type InflightSnapshot,
} from "./api";
import { activeApplies, downloadProgressByGroup, inflightCount, type ApplyTracker } from "./stores";
import {
  installNotificationsListener,
  pushNotification,
  refreshNotifications,
  type NotificationEntry,
  type NotificationKind,
} from "./notifications";

let installed = false;
let unlisteners: UnlistenFn[] = [];

function terminalKind(stage: string): NotificationKind | null {
  if (stage === "complete") return "apply_success";
  if (stage === "failed") return "apply_failure";
  if (stage === "cancelled") return "apply_cancelled";
  return null;
}

function buildTerminalEntry(
  p: ApplyProgress,
  tracker: ApplyTracker,
  kind: NotificationKind,
): NotificationEntry {
  const gameLabel = tracker.game_label || tracker.game_id;
  const family = tracker.family;
  const targetVersion = tracker.target_version;
  const title =
    kind === "apply_success"
      ? `${gameLabel} updated`
      : kind === "apply_failure"
        ? `${gameLabel} update failed`
        : `${gameLabel} update cancelled`;
  const body =
    kind === "apply_success"
      ? `${family} → ${targetVersion}`
      : kind === "apply_failure"
        ? (p.error ?? null)
        : null;
  return {
    id: crypto.randomUUID(),
    kind,
    title,
    body,
    created_at: new Date().toISOString(),
    read_at: null,
    dismissed_at: null,
    apply_id: p.apply_id,
    game_id: tracker.game_id,
    error_class: p.error_class ?? null,
    link: null,
  };
}

export async function installApplyEventListeners(): Promise<void> {
  if (installed) return;
  installed = true;
  const u1 = await listen<ApplyProgress>(APPLY_PROGRESS_EVENT, (event) => {
    const p = event.payload;
    activeApplies.update((map) => {
      const existing = map[p.apply_id];
      if (!existing) return map;
      const stage = p.stage;
      const isTerminal = stage === "complete" || stage === "failed" || stage === "cancelled";
      const failed_at =
        stage === "failed" ? (existing.failed_at_stage ?? existing.stage) : existing.failed_at_stage;
      const ended_at = isTerminal ? Date.now() : existing.ended_at;
      const next: ApplyTracker = {
        ...existing,
        group_id: existing.group_id || p.group_id,
        stage,
        failed_at_stage: failed_at,
        message: p.message,
        progress: p.progress,
        error: p.error ?? null,
        error_class: p.error_class ?? existing.error_class,
        attempt: p.attempt ?? existing.attempt,
        ended_at,
      };
      if (isTerminal) {
        const kind = terminalKind(stage);
        if (kind) {
          const entry = buildTerminalEntry(p, next, kind);
          pushNotification(entry).catch((err) =>
            console.warn("[dlssync] notification push failed:", err),
          );
        }
      }
      return { ...map, [p.apply_id]: next };
    });
  });
  const u2 = await listen<GroupDownloadProgress>(DOWNLOAD_PROGRESS_EVENT, (event) => {
    const p = event.payload;
    const now = Date.now();
    downloadProgressByGroup.update((m) => ({
      ...m,
      [p.group_id]: {
        group_id: p.group_id,
        url: p.url,
        bytes_downloaded: p.bytes_downloaded,
        bytes_total: p.bytes_total,
        bytes_per_sec: p.bytes_per_sec,
        attempt: p.attempt,
        last_update: now,
      },
    }));
    activeApplies.update((m) => {
      const next: Record<string, ApplyTracker> = { ...m };
      for (const id of Object.keys(next)) {
        const cur = next[id];
        if (cur.group_id === p.group_id && cur.stage === "download") {
          next[id] = {
            ...cur,
            bytes_downloaded: p.bytes_downloaded,
            bytes_total: p.bytes_total,
            bytes_per_sec: p.bytes_per_sec,
            attempt: p.attempt,
          };
        }
      }
      return next;
    });
  });
  const u3 = await listen<InflightSnapshot>(APPLY_INFLIGHT_EVENT, (event) => {
    inflightCount.set(event.payload.in_flight);
  });
  const u4 = await listen<void>(TRAY_CHECK_UPDATE_EVENT, () => {
    window.dispatchEvent(
      new CustomEvent("dlssync:check-updates", { detail: { force: true } }),
    );
  });
  const u5 = await installNotificationsListener();
  unlisteners = [u1, u2, u3, u4, u5];
  void refreshNotifications();
}

export async function uninstallApplyEventListeners(): Promise<void> {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
    }
  }
  unlisteners = [];
  installed = false;
}
