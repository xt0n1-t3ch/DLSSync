import { get } from "svelte/store";
import {
  applyUpdateBatch,
  applyUpdate,
  applyStreamlineSet,
  applyDllSet,
  cancelApply as cancelApplyApi,
  cancelAllApplies as cancelAllAppliesApi,
  type ApplyRequest,
  type ApplyBatchResult,
  type StreamlineSetResult,
  type DllRecord,
} from "./api";
import {
  activeApplies,
  downloadProgressByGroup,
  formatError,
  showToast,
  type ApplyTracker,
  type Toast,
} from "./stores";
import { familyVendor, familyCatalogKey, launcherLabel } from "./labels";
import { translate, locale } from "./i18n/index";
import { notifyApplySuccess } from "./community";
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
const ENDED_TRACKER_TTL_MS = 5 * 60 * 1000;

/// True when any tracker in the store is still running (no `ended_at`). Mirrors the
/// `isApplyInflight` guard used by the background daemon so a second dispatch never
/// clobbers an in-flight batch's trackers.
export function isApplyInflight(): boolean {
  return Object.values(get(activeApplies)).some((t) => t.ended_at === null);
}

/// Drops trackers that finished more than `ENDED_TRACKER_TTL_MS` ago plus any
/// download-progress entry whose group no longer backs a live tracker. Runs at
/// every new dispatch so a long tray session with daily auto-applies cannot
/// accrete state forever.
export function pruneEndedApplyState(now = Date.now()): void {
  const cutoff = now - ENDED_TRACKER_TTL_MS;
  activeApplies.update((m) => {
    const next: Record<string, ApplyTracker> = {};
    for (const [id, tracker] of Object.entries(m)) {
      if (tracker.ended_at === null || tracker.ended_at > cutoff) next[id] = tracker;
    }
    return next;
  });
  const liveGroups = new Set(
    Object.values(get(activeApplies))
      .map((t) => t.group_id)
      .filter((g) => g !== ""),
  );
  downloadProgressByGroup.update((m) => {
    const next: typeof m = {};
    for (const [group, state] of Object.entries(m)) {
      if (liveGroups.has(group)) next[group] = state;
    }
    return next;
  });
}

export async function dispatchApply(
  targets: ApplyTarget[],
  opts: DispatchOptions = {},
): Promise<ApplyBatchResult | null> {
  const loc = get(locale);
  if (targets.length === 0) {
    (opts.toast ?? DEFAULT_TOAST)("warning", translate(loc, "toast.nothingSelected"));
    return null;
  }
  if (isApplyInflight()) {
    (opts.toast ?? DEFAULT_TOAST)("warning", translate(loc, "toast.applyInProgress"));
    return null;
  }
  pruneEndedApplyState();
  const { trackers, requests } = prepareApply(targets);
  activeApplies.update((m) => ({ ...m, ...trackers }));
  opts.showModal?.();
  const toast = opts.toast ?? DEFAULT_TOAST;
  const uniqueGames = new Set(targets.map((t) => t.game_id)).size;
  toast(
    "info",
    translate(loc, "toast.queuedAcross", {
      count: targets.length,
      updates: translate(loc, "toast.queuedUpdates", { count: targets.length }),
      games: translate(loc, "toast.queuedGames", { count: uniqueGames }),
    }),
  );
  try {
    const result = await applyUpdateBatch({ items: requests });
    annotateOutcomes(result);
    notifyApplySuccess(result.outcomes.filter((o) => o.success).length);
    return result;
  } catch (err: unknown) {
    const msg = formatError(err);
    failAllTrackers(trackers, msg);
    toast("danger", translate(loc, "toast.batchApplyFailed", { msg }));
    return null;
  }
}

function prepareApply(targets: ApplyTarget[]): {
  trackers: Record<string, ApplyTracker>;
  requests: ApplyRequest[];
} {
  const loc = get(locale);
  const trackers: Record<string, ApplyTracker> = {};
  const requests: ApplyRequest[] = [];
  for (const t of targets) {
    const apply_id = crypto.randomUUID();
    trackers[apply_id] = {
      apply_id,
      group_id: "",
      game_id: t.game_id,
      game_label: t.game_label,
      dll_path: t.record.path,
      family: t.record.family,
      target_version: t.target_version,
      stage: "download",
      failed_at_stage: null,
      message: translate(loc, "toast.trackerQueued"),
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
  return { trackers, requests };
}

function failAllTrackers(trackers: Record<string, ApplyTracker>, msg: string): void {
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
}

/// Apply an NVIDIA Streamline plugin set as one atomic transaction (all-or-nothing
/// in the backend). Reuses the per-member tracker/modal plumbing so the progress
/// modal shows each file, but the whole set succeeds or rolls back together.
export async function dispatchStreamlineSet(
  targets: ApplyTarget[],
  opts: DispatchOptions = {},
): Promise<StreamlineSetResult | null> {
  const toast = opts.toast ?? DEFAULT_TOAST;
  const loc = get(locale);
  if (targets.length === 0) {
    toast("warning", translate(loc, "toast.streamlineNoUpdates"));
    return null;
  }
  if (isApplyInflight()) {
    toast("warning", translate(loc, "toast.applyInProgress"));
    return null;
  }
  pruneEndedApplyState();
  const { trackers, requests } = prepareApply(targets);
  activeApplies.update((m) => ({ ...m, ...trackers }));
  opts.showModal?.();
  const count = targets.length;
  toast("info", translate(loc, "toast.streamlineUpdating", { count }));
  try {
    const result = await applyStreamlineSet(requests);
    if (result.success) {
      annotateOutcomes({ outcomes: result.applied });
      toast("success", translate(loc, "toast.streamlineUpdated", { count }));
      notifyApplySuccess(result.applied.length);
    } else {
      const rolledBack = result.rolled_back ? translate(loc, "toast.streamlineRolledBack") : "";
      const reason = result.error ?? translate(loc, "toast.streamlineSetFailed");
      failAllTrackers(trackers, `${reason}${rolledBack}`);
      toast(
        "danger",
        translate(loc, "toast.streamlineUpdateFailed", {
          error: result.error ?? translate(loc, "toast.unknownError"),
          rolledBack,
        }),
      );
    }
    return result;
  } catch (err: unknown) {
    const msg = formatError(err);
    failAllTrackers(trackers, msg);
    toast("danger", translate(loc, "toast.streamlineApplyFailed", { msg }));
    return null;
  }
}

/// Apply a coherent multi-DLL vendor set (FSR SDK / XeSS SDK) as one atomic
/// transaction. Mirrors `dispatchStreamlineSet` but routes through the
/// generalized `apply_dll_set` command, whose backend guard enforces set
/// coherence plus the FSR4 hardware gate (fail-closed).
export async function dispatchDllSet(
  targets: ApplyTarget[],
  setLabel: string,
  opts: DispatchOptions = {},
): Promise<StreamlineSetResult | null> {
  const toast = opts.toast ?? DEFAULT_TOAST;
  const loc = get(locale);
  if (targets.length === 0) {
    toast("warning", translate(loc, "toast.setNoUpdates", { label: setLabel }));
    return null;
  }
  if (isApplyInflight()) {
    toast("warning", translate(loc, "toast.applyInProgress"));
    return null;
  }
  pruneEndedApplyState();
  const { trackers, requests } = prepareApply(targets);
  activeApplies.update((m) => ({ ...m, ...trackers }));
  opts.showModal?.();
  const count = targets.length;
  toast("info", translate(loc, "toast.setUpdating", { label: setLabel, count }));
  try {
    const result = await applyDllSet(requests);
    if (result.success) {
      annotateOutcomes({ outcomes: result.applied });
      toast("success", translate(loc, "toast.setUpdated", { label: setLabel, count }));
      notifyApplySuccess(result.applied.length);
    } else {
      const rolledBack = result.rolled_back ? translate(loc, "toast.streamlineRolledBack") : "";
      const reason = result.error ?? translate(loc, "toast.setFailed", { label: setLabel });
      failAllTrackers(trackers, `${reason}${rolledBack}`);
      toast(
        "danger",
        translate(loc, "toast.setUpdateFailed", {
          label: setLabel,
          error: result.error ?? translate(loc, "toast.unknownError"),
          rolledBack,
        }),
      );
    }
    return result;
  } catch (err: unknown) {
    const msg = formatError(err);
    failAllTrackers(trackers, msg);
    toast("danger", translate(loc, "toast.setApplyFailed", { label: setLabel, msg }));
    return null;
  }
}

export async function retrySingleApply(tracker: ApplyTracker): Promise<void> {
  if (isApplyInflight()) {
    showToast("warning", translate(get(locale), "toast.applyInProgress"));
    return;
  }
  const retrying = translate(get(locale), "toast.trackerRetrying");
  activeApplies.update((m) => {
    const cur = m[tracker.apply_id];
    if (!cur) return m;
    return {
      ...m,
      [tracker.apply_id]: {
        ...cur,
        stage: "download",
        failed_at_stage: null,
        message: retrying,
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
  if (isApplyInflight()) {
    showToast("warning", translate(get(locale), "toast.applyInProgress"));
    return;
  }
  const items: ApplyRequest[] = failed.map((t) => ({
    apply_id: t.apply_id,
    game_id: t.game_id,
    game_label: t.game_label,
    dll_path: t.dll_path,
    vendor: familyVendor(t.family as DllRecord["family"]),
    family: familyCatalogKey(t.family as DllRecord["family"]),
    target_version: t.target_version,
  }));
  const retrying = translate(get(locale), "toast.trackerRetrying");
  activeApplies.update((m) => {
    const next = { ...m };
    for (const t of failed) {
      const cur = next[t.apply_id];
      if (!cur) continue;
      next[t.apply_id] = {
        ...cur,
        stage: "download",
        failed_at_stage: null,
        message: retrying,
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
  try {
    await cancelApplyApi(applyId);
  } catch (err: unknown) {
    showToast("danger", translate(get(locale), "toast.cancelFailed", { msg: formatError(err) }));
  }
}

export async function cancelAll(): Promise<void> {
  try {
    await cancelAllAppliesApi();
  } catch (err: unknown) {
    showToast("danger", translate(get(locale), "toast.cancelFailed", { msg: formatError(err) }));
  }
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
  const loc = get(locale);
  activeApplies.update((m) => {
    const next = { ...m };
    for (const o of result.outcomes) {
      const cur = next[o.apply_id];
      if (!cur) continue;
      if (o.success) {
        next[o.apply_id] = {
          ...cur,
          stage: "complete",
          message: o.new_version
            ? translate(loc, "toast.trackerUpdatedTo", { version: o.new_version })
            : translate(loc, "toast.trackerUpdated"),
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
