import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import type { SystemDriverOutcome, SystemDriverUpdate } from "@/lib/api";

let pendingResolve: ((outcome: SystemDriverOutcome) => void) | null = null;
const { scanSpy } = vi.hoisted(() => ({ scanSpy: vi.fn(async () => []) }));

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    installSystemDriver: vi.fn(
      () =>
        new Promise<SystemDriverOutcome>((resolve) => {
          pendingResolve = resolve;
        }),
    ),
    scanSystemDrivers: scanSpy,
  };
});

import {
  systemDriverInstall,
  startSystemDriverInstall,
  applySystemDriverProgress,
  toasts,
} from "@/lib/stores";

function update(id: string): SystemDriverUpdate {
  return {
    update_id: id,
    title: "Realtek - MEDIA - 6.0.9600.1",
    class: "audio",
    provider: "Realtek Semiconductor Corp.",
    driver_version: "6.0.9600.1",
    driver_date: "2026-04-10",
    hardware_id: "HDAUDIO\\FUNC_01&VEN_10EC&DEV_0256",
    size_bytes: 12_345_678,
    target_device: "Realtek High Definition Audio",
  };
}

function complete(outcome: SystemDriverOutcome): void {
  expect(pendingResolve, "install_system_driver was invoked").not.toBeNull();
  pendingResolve?.(outcome);
  pendingResolve = null;
}

beforeEach(() => {
  systemDriverInstall.set({ updateId: null, stage: null, message: "", fraction: null });
  toasts.set([]);
  pendingResolve = null;
  scanSpy.mockClear();
});

describe("system driver install — shared store state machine", () => {
  it("ignores progress events while no install is active", () => {
    applySystemDriverProgress({ stage: "downloading", message: "stray", fraction: 0.4 });
    expect(get(systemDriverInstall).updateId).toBeNull();
    expect(get(systemDriverInstall).stage).toBeNull();
  });

  it("survives a view change: progress keeps updating the store mid-download", async () => {
    const p = startSystemDriverInstall(update("u-1:1"));
    expect(get(systemDriverInstall).updateId).toBe("u-1:1");
    expect(get(systemDriverInstall).stage).toBe("downloading");

    applySystemDriverProgress({ stage: "downloading", message: "Downloading driver", fraction: 0.5 });
    expect(get(systemDriverInstall).fraction).toBe(0.5);
    applySystemDriverProgress({ stage: "installing", message: "Installing", fraction: null });
    expect(get(systemDriverInstall).stage).toBe("installing");

    complete({ success: true, reboot_required: false, result_code: 2, message: "ok" });
    await p;
    expect(get(systemDriverInstall).updateId).toBeNull();
    expect(get(toasts).at(-1)?.kind).toBe("success");
    expect(scanSpy).toHaveBeenCalledTimes(1);
  });

  it("only one install runs at a time", async () => {
    const first = startSystemDriverInstall(update("u-1:1"));
    expect(get(systemDriverInstall).updateId).toBe("u-1:1");
    await startSystemDriverInstall(update("u-2:1"));
    expect(get(systemDriverInstall).updateId).toBe("u-1:1");
    complete({ success: true, reboot_required: false, result_code: 2, message: "done" });
    await first;
  });

  it("surfaces a reboot hint in the success toast", async () => {
    const p = startSystemDriverInstall(update("u-1:1"));
    complete({ success: true, reboot_required: true, result_code: 2, message: "ok" });
    await p;
    expect(get(toasts).at(-1)?.kind).toBe("success");
    expect(get(toasts).at(-1)?.message).toMatch(/restart/i);
  });

  it("parks a failed install in a VISIBLE terminal state (not a silent reset) + danger toast", async () => {
    const p = startSystemDriverInstall(update("u-1:1"));
    complete({ success: false, reboot_required: false, result_code: 4, message: "Windows Update returned result code 4." });
    await p;
    const s = get(systemDriverInstall);
    expect(s.updateId).toBe("u-1:1");
    expect(s.stage).toBe("failed");
    expect(s.message).toMatch(/result code 4/);
    expect(get(toasts).at(-1)?.kind).toBe("danger");
    expect(scanSpy).not.toHaveBeenCalled();
  });
});
