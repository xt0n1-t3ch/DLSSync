import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { showToast, dismissToast, toasts } from "@/lib/stores";

function get<T>(store: { subscribe: (run: (v: T) => void) => () => void }): T {
  let value!: T;
  const unsub = store.subscribe((v) => (value = v));
  unsub();
  return value;
}

beforeEach(() => {
  toasts.set([]);
  vi.useFakeTimers();
});
afterEach(() => {
  vi.runOnlyPendingTimers();
  vi.useRealTimers();
});

describe("toast popup store (the data the Toast popup renders)", () => {
  it("appends a toast with kind + message and a unique id", () => {
    showToast("success", "Updated Cyberpunk 2077");
    const arr = get(toasts);
    expect(arr.length).toBe(1);
    expect(arr[0]).toMatchObject({ kind: "success", message: "Updated Cyberpunk 2077" });
    expect(typeof arr[0].id).toBe("number");
  });

  it("stacks multiple toasts in FIFO order with distinct ids", () => {
    showToast("info", "one");
    showToast("danger", "two");
    const arr = get(toasts);
    expect(arr.map((t) => t.message)).toEqual(["one", "two"]);
    expect(arr[0].id).not.toBe(arr[1].id);
  });

  it("auto-dismisses after the ttl elapses", () => {
    showToast("warning", "transient", 4000);
    expect(get(toasts).length).toBe(1);
    vi.advanceTimersByTime(3999);
    expect(get(toasts).length).toBe(1);
    vi.advanceTimersByTime(1);
    expect(get(toasts).length).toBe(0);
  });

  it("dismiss removes only the targeted toast", () => {
    showToast("info", "keep", 999999);
    showToast("danger", "drop", 999999);
    const dropId = get(toasts)[1].id;
    dismissToast(dropId);
    const arr = get(toasts);
    expect(arr.length).toBe(1);
    expect(arr[0].message).toBe("keep");
  });

  it("dismissing an unknown id is a no-op", () => {
    showToast("info", "stays", 999999);
    dismissToast(-1);
    expect(get(toasts).length).toBe(1);
  });
});
