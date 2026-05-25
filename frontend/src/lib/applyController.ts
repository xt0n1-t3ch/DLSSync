import { get } from "svelte/store";
import {
  applyUpdateBatch,
  applyUpdate,
  cancelApply as cancelApplyApi,
  cancelAllApplies as cancelAllAppliesApi,
  type ApplyRequest,
  type ApplyBatchResult,
  type DllRecord,
} from "./api";
import {
  activeApplies,
  showToast,
  type ApplyTracker,
  type Toast,
} from "./stores";
import { familyVendor, familyCatalogKey, launcherLabel } from "./labels";
import type { LauncherKind } from "./api";

export interface ApplyTarget {
  game_id: string;
  game_label: string;
  record: DllRecord;
  target_version: string;
}

export interface DispatchOptions {
  showModal?: () => void;
  toast?: (kind: Toast["kind"], message: string) => void;
}

const DEFAULT_TOAST = (kind: Toast["kind"], message: string): void => showToast(kind, message);

export async function dispatchApply(
  targets: ApplyTarget[],
  opts: DispatchOptions = {},
): Promise<ApplyBatchResult | null> {
  if (targets.length === 0) {
    (opts.toast ?? DEFAULT_TOAST)("warning", "Nothing selected");
    return null;
  }
  const trackers: Record<string, ApplyTracker> = {};
  const requests: ApplyRequest[] = [];
  for (const t of targets) {
    const apply_id = crypto.randomUUID();
    const tracker: ApplyTracker = {
      apply_id,
      group_id: "",
      game_id: t.game_id,
      game_label: t.game_label,
      dll_path: t.record.path,
      family: t.record.family,
      target_version: t.target_version,
      stage: "download",
      failed_at_stage: null,
      message: "Queued",
      progress: null,
      error: null,
      error_class: null,
      attempt: null,
      bytes_downloaded: 0,
      bytes_total: null,
      bytes_per_sec: 0,
      started_at: Date.now(),
      ended_at: null,
    };
    trackers[apply_id] = tracker;
    requests.push({
      apply_id,
      game_id: t.game_id,
      game_label: t.game_label,
      dll_path: t.record.path,
      vendor: familyVendor(t.record.family),
      family: familyCatalogKey(t.record.family),
      target_version: t.target_version,
    });
  }
  activeApplies.set(trackers);
  opts.showModal?.();
  const toast = opts.toast ?? DEFAULT_TOAST;
  const uniqueGames = new Set(targets.map((t) => t.game_id)).size;
  toast(
    "info",
    `Queued ${targets.length} update${targets.length === 1 ? "" : "s"} across ${uniqueGames} game${uniqueGames === 1 ? "" : "s"}`,
  );
  try {
    const result = await applyUpdateBatch({ items: requests });
    annotateOutcomes(result);
    return result;
  } catch (err: unknown) {
    const msg = formatError(err);
    activeApplies.update((m) => {
      const next = { ...m };
      for (const id of Object.keys(trackers)) {
        const cur = next[id];
        if (!cur) continue;
        next[id] = {
          ...cur,
          stage: "failed",
          failed_at_stage: cur.failed_at_stage ?? cur.stage,
          error: msg,
          message: msg,
          ended_at: Date.now(),
        };
      }
      return next;
    });
    toast("danger", `Batch apply failed: ${msg}`);
    return null;
  }
}

export async function retrySingleApply(tracker: ApplyTracker): Promise<void> {
  activeApplies.update((m) => {
    const cur = m[tracker.apply_id];
    if (!cur) return m;
    return {
      ...m,
      [tracker.apply_id]: {
        ...cur,
        stage: "download",
        failed_at_stage: null,
        message: "Retrying…",
        error: null,
        error_class: null,
        progress: 0,
        ended_at: null,
        bytes_downloaded: 0,
      },
    };
  });
  try {
    await applyUpdate({
      apply_id: tracker.apply_id,
      game_id: tracker.game_id,
      game_label: tracker.game_label,
      dll_path: tracker.dll_path,
      vendor: familyVendor(tracker.family as DllRecord["family"]),
      family: familyCatalogKey(tracker.family as DllRecord["family"]),
      target_version: tracker.target_version,
    });
  } catch (err: unknown) {
    const msg = formatError(err);
    activeApplies.update((m) => {
      const cur = m[tracker.apply_id];
      if (!cur) return m;
      return {
        ...m,
        [tracker.apply_id]: {
          ...cur,
          stage: "failed",
          failed_at_stage: cur.failed_at_stage ?? cur.stage,
          error: msg,
          message: msg,
          ended_at: Date.now(),
        },
      };
    });
  }
}

export async function retryFailedTrackers(trackers: ApplyTracker[]): Promise<void> {
  const failed = trackers.filter((t) => t.stage === "failed" || t.stage === "cancelled");
  if (failed.length === 0) return;
  const items: ApplyRequest[] = failed.map((t) => ({
    apply_id: t.apply_id,
    game_id: t.game_id,
    game_label: t.game_label,
    dll_path: t.dll_path,
    vendor: familyVendor(t.family as DllRecord["family"]),
    family: familyCatalogKey(t.family as DllRecord["family"]),
    target_version: t.target_version,
  }));
  activeApplies.update((m) => {
    const next = { ...m };
    for (const t of failed) {
      const cur = next[t.apply_id];
      if (!cur) continue;
      next[t.apply_id] = {
        ...cur,
        stage: "download",
        failed_at_stage: null,
        message: "Retrying…",
        error: null,
        error_class: null,
        progress: 0,
        ended_at: null,
        bytes_downloaded: 0,
      };
    }
    return next;
  });
  try {
    const result = await applyUpdateBatch({ items });
    annotateOutcomes(result);
  } catch (err: unknown) {
    const msg = formatError(err);
    activeApplies.update((m) => {
      const next = { ...m };
      for (const t of failed) {
        const cur = next[t.apply_id];
        if (!cur) continue;
        next[t.apply_id] = {
          ...cur,
          stage: "failed",
          failed_at_stage: cur.failed_at_stage ?? cur.stage,
          error: msg,
          message: msg,
          ended_at: Date.now(),
        };
      }
      return next;
    });
  }
}

export async function cancelOne(applyId: string): Promise<void> {
  await cancelApplyApi(applyId);
}

export async function cancelAll(): Promise<void> {
  await cancelAllAppliesApi();
}

export function snapshotActive(): ApplyTracker[] {
  return Object.values(get(activeApplies));
}

export function buildTargetFromRecord(
  game: { id: string; name: string; launcher: LauncherKind | string },
  record: DllRecord,
  target_version: string,
): ApplyTarget {
  return {
    game_id: game.id,
    game_label: `${launcherLabel(game.launcher as LauncherKind)} - ${game.name}`,
    record,
    target_version,
  };
}

function annotateOutcomes(result: ApplyBatchResult): void {
  activeApplies.update((m) => {
    const next = { ...m };
    for (const o of result.outcomes) {
      const cur = next[o.apply_id];
      if (!cur) continue;
      if (o.success) {
        next[o.apply_id] = {
          ...cur,
          stage: "complete",
          message: o.new_version ? `Updated to v${o.new_version}` : "Updated",
          progress: 1,
          ended_at: Date.now(),
        };
      } else if (!cur.error && o.error) {
        next[o.apply_id] = {
          ...cur,
          stage: "failed",
          failed_at_stage: cur.failed_at_stage ?? cur.stage,
          error: o.error,
          message: o.error,
          ended_at: Date.now(),
        };
      } else if (cur.stage !== "complete" && cur.stage !== "failed") {
        next[o.apply_id] = {
          ...cur,
          ended_at: Date.now(),
        };
      }
    }
    return next;
  });
}

function formatError(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}
