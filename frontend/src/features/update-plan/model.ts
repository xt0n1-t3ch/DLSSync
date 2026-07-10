import { get, writable } from "svelte/store";
import type { ApplyTarget } from "../../lib/applyController";

export interface ReviewedUpdatePlan {
  targets: ApplyTarget[];
  catalogGeneratedAt: string;
}

interface PendingPlan {
  targets: ApplyTarget[];
  resolve: (result: ReviewedUpdatePlan | null) => void;
}

export const pendingUpdatePlan = writable<PendingPlan | null>(null);

export function reviewUpdatePlan(targets: ApplyTarget[]): Promise<ReviewedUpdatePlan | null> {
  if (get(pendingUpdatePlan)) return Promise.resolve(null);
  return new Promise((resolve) => pendingUpdatePlan.set({ targets, resolve }));
}

export function completeUpdatePlan(result: ReviewedUpdatePlan | null): void {
  const pending = get(pendingUpdatePlan);
  if (!pending) return;
  pendingUpdatePlan.set(null);
  pending.resolve(result);
}
