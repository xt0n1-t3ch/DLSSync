import { describe, it, expect, beforeEach, vi } from "vitest";
import { get, writable } from "svelte/store";
import { optimisticToggle, toasts } from "@/lib/stores";

beforeEach(() => {
  toasts.set([]);
});

describe("optimisticToggle — reversible apply with quiet-undo", () => {
  it("applies the optimistic state immediately and shows an Undo toast", async () => {
    const flag = writable(false);
    await optimisticToggle({
      applyOptimistic: () => flag.set(true),
      revert: () => flag.set(false),
      commit: () => Promise.resolve(),
      message: "Feature disabled",
    });
    expect(get(flag)).toBe(true);
    const t = get(toasts).at(-1);
    expect(t?.kind).toBe("info");
    expect(t?.message).toBe("Feature disabled");
    expect(t?.action?.label).toBe("Undo");
  });

  it("clicking Undo reverts the optimistic state and dismisses the toast", async () => {
    const flag = writable(false);
    await optimisticToggle({
      applyOptimistic: () => flag.set(true),
      revert: () => flag.set(false),
      commit: () => Promise.resolve(),
      message: "Feature disabled",
    });
    expect(get(flag)).toBe(true);
    const t = get(toasts).at(-1);
    t?.action?.run();
    expect(get(flag)).toBe(false);
    expect(get(toasts).length).toBe(0);
  });

  it("reverts and surfaces a danger toast when the backend commit rejects", async () => {
    const flag = writable(false);
    await optimisticToggle({
      applyOptimistic: () => flag.set(true),
      revert: () => flag.set(false),
      commit: () => Promise.reject(new Error("disk full")),
      message: "Feature disabled",
    });
    expect(get(flag)).toBe(false);
    const last = get(toasts).at(-1);
    expect(last?.kind).toBe("danger");
    expect(last?.message).toMatch(/disk full/);
  });

  it("a backend failure AFTER the user already undid does not double-revert", async () => {
    const revert = vi.fn();
    let rejectCommit!: (e: unknown) => void;
    const pending = optimisticToggle({
      applyOptimistic: () => undefined,
      revert,
      commit: () => new Promise<void>((_, reject) => { rejectCommit = reject; }),
      message: "Feature disabled",
    });
    get(toasts).at(-1)?.action?.run();
    expect(revert).toHaveBeenCalledTimes(1);
    rejectCommit(new Error("late failure"));
    await pending;
    expect(revert).toHaveBeenCalledTimes(1);
    expect(get(toasts).some((t) => t.kind === "danger")).toBe(false);
  });
});
