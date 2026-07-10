import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import type { ApplyBatchResult, DllRecord } from "@/lib/api";

let pendingResolve: ((result: ApplyBatchResult) => void) | null = null;
const applyUpdateBatch = vi.fn(
  () =>
    new Promise<ApplyBatchResult>((resolve) => {
      pendingResolve = resolve;
    }),
);

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    getCatalogStatus: vi.fn(async () => ({ provenance: { generated_at: "2026-07-10T00:00:00Z" } })),
    applyUpdateBatch: (...a: unknown[]) =>
      (applyUpdateBatch as unknown as (...x: unknown[]) => Promise<ApplyBatchResult>)(...a),
  };
});

vi.mock("@/lib/community", () => ({ notifyApplySuccess: vi.fn() }));

import { dispatchApply, type ApplyTarget } from "@/lib/applyController";
import { activeApplies, toasts } from "@/lib/stores";

const reviewPlan = async (targets: ApplyTarget[]) => ({
  targets,
  catalogGeneratedAt: "2026-07-10T00:00:00Z",
});

function target(gameId: string, family: DllRecord["family"] = "dlss_sr"): ApplyTarget {
  return {
    game_id: gameId,
    game_label: gameId,
    record: {
      family,
      path: `C:\\Games\\${gameId}\\nvngx_dlss.dll`,
      current_version: "1.0.0.0",
      file_description: null,
      sha256: null,
    },
    target_version: "2.0.0.0",
  };
}

beforeEach(() => {
  pendingResolve = null;
  applyUpdateBatch.mockClear();
  activeApplies.set({});
  toasts.set([]);
});

describe("dispatchApply — concurrent dispatch never wipes an in-flight batch", () => {
  it("a second dispatch while the first is in flight preserves the first batch's trackers", async () => {
    const first = dispatchApply([target("alpha"), target("beta")], { reviewPlan });
    await Promise.resolve();
    await Promise.resolve();
    const afterFirst = get(activeApplies);
    expect(Object.keys(afterFirst)).toHaveLength(2);
    const firstIds = Object.keys(afterFirst);

    const second = await dispatchApply([target("gamma")], { reviewPlan });
    expect(second).toBeNull();
    expect(applyUpdateBatch).toHaveBeenCalledTimes(1);

    const afterSecond = get(activeApplies);
    expect(Object.keys(afterSecond).sort()).toEqual([...firstIds].sort());
    for (const id of firstIds) {
      expect(afterSecond[id]).toEqual(afterFirst[id]);
    }
    expect(get(toasts).at(-1)?.kind).toBe("warning");

    pendingResolve?.({ outcomes: [] });
    await first;
  });

  it("merges a fresh batch into recently-finished trackers rather than replacing them", async () => {
    const done = dispatchApply([target("alpha")], { reviewPlan });
    await Promise.resolve();
    await Promise.resolve();
    pendingResolve?.({ outcomes: [] });
    await done;
    // In the real app the apply-progress event listener stamps `ended_at`; the
    // unit test has no listener, so mark the batch finished explicitly to clear
    // the in-flight guard before the next dispatch.
    activeApplies.update((m) => {
      const next = { ...m };
      for (const id of Object.keys(next)) next[id] = { ...next[id], ended_at: Date.now() };
      return next;
    });
    const resolved = get(activeApplies);
    expect(Object.keys(resolved)).toHaveLength(1);
    const oldId = Object.keys(resolved)[0];

    dispatchApply([target("beta")], { reviewPlan });
    await Promise.resolve();
    await Promise.resolve();
    const merged = get(activeApplies);
    expect(Object.keys(merged)).toHaveLength(2);
    expect(merged[oldId]).toEqual(resolved[oldId]);

    pendingResolve?.({ outcomes: [] });
  });

  it("prunes stale finished trackers when a fresh batch dispatches", async () => {
    const done = dispatchApply([target("alpha")], { reviewPlan });
    await Promise.resolve();
    await Promise.resolve();
    pendingResolve?.({ outcomes: [] });
    await done;
    activeApplies.update((m) => {
      const next = { ...m };
      for (const id of Object.keys(next)) next[id] = { ...next[id], ended_at: 1 };
      return next;
    });
    const staleId = Object.keys(get(activeApplies))[0];

    dispatchApply([target("beta")], { reviewPlan });
    await Promise.resolve();
    await Promise.resolve();
    const merged = get(activeApplies);
    expect(Object.keys(merged)).toHaveLength(1);
    expect(merged[staleId]).toBeUndefined();

    pendingResolve?.({ outcomes: [] });
  });
});
