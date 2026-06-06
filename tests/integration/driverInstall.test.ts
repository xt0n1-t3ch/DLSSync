import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import type { DriverInstallOutcome, DriverStatusReport, GpuVendor } from "@/lib/api";

let pendingResolve: ((outcome: DriverInstallOutcome) => void) | null = null;

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    installDriver: vi.fn(
      () =>
        new Promise<DriverInstallOutcome>((resolve) => {
          pendingResolve = resolve;
        }),
    ),
    checkDriverUpdates: vi.fn(async () => []),
  };
});

import {
  driverInstall,
  driverRebootPending,
  loadDriverUpdates,
  startDriverInstall,
  applyDriverInstallProgress,
  toasts,
} from "@/lib/stores";
import { checkDriverUpdates } from "@/lib/api";

function report(vendor: GpuVendor, downloadUrl: string | null): DriverStatusReport {
  return {
    device: { class: "gpu", vendor, pci_vendor_id: 0, pci_device_id: 0, model: "Test GPU" },
    installed: { packed: 1, display: "1.0", raw: "1.0" },
    latest:
      downloadUrl === null
        ? null
        : {
            vendor,
            version: { packed: 2, display: "2.0", raw: "2.0" },
            channel: "stable",
            display_version: null,
            is_beta: false,
            download_url: downloadUrl,
            size_bytes: 100,
            signature_subject: "x",
            released_at: null,
            release_notes_url: "https://vendor/notes",
            changelog: null,
          },
    status: "update_available",
  };
}

function complete(outcome: DriverInstallOutcome): void {
  expect(pendingResolve, "install_driver was invoked").not.toBeNull();
  pendingResolve?.(outcome);
  pendingResolve = null;
}

beforeEach(() => {
  driverInstall.set({ vendor: null, stage: null, message: "", fraction: null });
  driverRebootPending.set({});
  toasts.set([]);
  pendingResolve = null;
  vi.mocked(checkDriverUpdates).mockResolvedValue([]);
});

describe("driver install — shared store state machine", () => {
  it("ignores progress events while no install is active", () => {
    applyDriverInstallProgress({ stage: "downloading", message: "stray", progress: 0.4 });
    expect(get(driverInstall).vendor).toBeNull();
    expect(get(driverInstall).stage).toBeNull();
  });

  it("survives a view change: progress keeps updating the store mid-download", async () => {
    const p = startDriverInstall(report("nvidia", "https://nv/driver.exe"));
    expect(get(driverInstall).vendor).toBe("nvidia");
    expect(get(driverInstall).stage).toBe("downloading");

    applyDriverInstallProgress({ stage: "downloading", message: "Downloading driver", progress: 0.5 });
    expect(get(driverInstall).fraction).toBe(0.5);
    applyDriverInstallProgress({ stage: "verifying", message: "Verifying signature", progress: null });
    expect(get(driverInstall).stage).toBe("verifying");
    applyDriverInstallProgress({ stage: "installing", message: "Installing", progress: null });
    expect(get(driverInstall).stage).toBe("installing");

    complete({ stage: "completed", exit_code: 0, message: "Driver installed successfully.", reboot_required: false });
    await p;
    expect(get(driverInstall).vendor).toBeNull();
    expect(get(toasts).at(-1)?.kind).toBe("success");
  });

  it("only one install runs at a time", async () => {
    const first = startDriverInstall(report("intel", "https://intel/gfx.exe"));
    expect(get(driverInstall).vendor).toBe("intel");
    await startDriverInstall(report("nvidia", "https://nv/driver.exe"));
    expect(get(driverInstall).vendor).toBe("intel");
    complete({ stage: "completed", exit_code: 0, message: "done", reboot_required: false });
    await first;
  });

  it("maps a cancelled outcome to a warning toast and clears state", async () => {
    const p = startDriverInstall(report("intel", "https://intel/gfx.exe"));
    complete({ stage: "cancelled", exit_code: 1602, message: "Installation cancelled.", reboot_required: false });
    await p;
    expect(get(driverInstall).vendor).toBeNull();
    expect(get(toasts).at(-1)?.kind).toBe("warning");
  });

  it("maps a failed outcome (e.g. Intel exit 8) to a danger toast and clears state", async () => {
    const p = startDriverInstall(report("intel", "https://intel/gfx.exe"));
    complete({ stage: "failed", exit_code: 8, message: "This Intel driver does not list your GPU (exit code 8).", reboot_required: false });
    await p;
    expect(get(driverInstall).vendor).toBeNull();
    expect(get(toasts).at(-1)?.kind).toBe("danger");
  });

  it("does nothing for a report without a direct download URL (AMD open-page path)", async () => {
    await startDriverInstall(report("amd", ""));
    expect(get(driverInstall).vendor).toBeNull();
    expect(pendingResolve).toBeNull();
  });
});

describe("driver install — reboot-pending state", () => {
  it("a completed install with reboot_required marks the vendor pending with its version", async () => {
    const p = startDriverInstall(report("nvidia", "https://nv/driver.exe"));
    complete({ stage: "completed", exit_code: 3010, message: "Driver installed. Restart to finish.", reboot_required: true });
    await p;
    expect(get(driverRebootPending).nvidia).toBe("2.0");
    expect(get(driverInstall).vendor).toBeNull();
    expect(get(toasts).at(-1)?.kind).toBe("success");
  });

  it("a completed install without reboot_required clears any pending state for the vendor", async () => {
    driverRebootPending.set({ nvidia: "1.9" });
    const p = startDriverInstall(report("nvidia", "https://nv/driver.exe"));
    complete({ stage: "completed", exit_code: 0, message: "Driver installed successfully.", reboot_required: false });
    await p;
    expect(get(driverRebootPending).nvidia).toBeUndefined();
  });

  it("loadDriverUpdates prunes a vendor that comes back up_to_date", async () => {
    driverRebootPending.set({ nvidia: "2.0" });
    const upToDate = report("nvidia", "https://nv/driver.exe");
    upToDate.status = "up_to_date";
    vi.mocked(checkDriverUpdates).mockResolvedValueOnce([upToDate]);
    await loadDriverUpdates();
    expect(get(driverRebootPending).nvidia).toBeUndefined();
  });

  it("loadDriverUpdates keeps a vendor still showing update_available (staged, not active)", async () => {
    driverRebootPending.set({ nvidia: "2.0" });
    vi.mocked(checkDriverUpdates).mockResolvedValueOnce([report("nvidia", "https://nv/driver.exe")]);
    await loadDriverUpdates();
    expect(get(driverRebootPending).nvidia).toBe("2.0");
  });
});
