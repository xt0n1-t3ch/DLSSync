import { describe, it, expect } from "vitest";
import { makeNotificationEntry, vendorKeyForNotification } from "@/lib/notifications";
import type { NotificationKind } from "@/lib/notifications";

const resolve = (
  kind: NotificationKind,
  title: string,
  body: string | null = null,
  vendor: string | null = null,
): string | null => vendorKeyForNotification(makeNotificationEntry(kind, title, body, { vendor }));

describe("vendorKeyForNotification — structured field is authoritative", () => {
  it("returns the stamped vendor verbatim for any kind", () => {
    expect(resolve("apply_success", "Updated Cyberpunk 2077", "DLSS FG 310.6", "nvidia")).toBe("nvidia");
    expect(resolve("backup_restored", "Restored backup", null, "amd")).toBe("amd");
    expect(resolve("catalog_update_available", "XeSS 2.0", null, "intel")).toBe("intel");
  });

  it("the stamped vendor wins over conflicting text", () => {
    expect(resolve("apply_success", "Updated game", "NVIDIA DLSS applied", "amd")).toBe("amd");
  });
});

describe("vendorKeyForNotification — the app's own name never reads as a vendor", () => {
  it("does not match nvidia via the substring 'dlss' inside 'DLSSync'", () => {
    expect(resolve("app_update_available", "DLSSync v1.6.4 available", "A visual overhaul")).toBeNull();
  });

  it("stays null even if 'DLSSync' lands in a vendor-bound kind", () => {
    expect(resolve("catalog_update_available", "DLSSync update", "new build")).toBeNull();
  });
});

describe("vendorKeyForNotification — app/scan/refresh kinds are never vendor-branded", () => {
  it("returns null for app_update / scan_failed / catalog_refresh_failed despite vendor text", () => {
    expect(resolve("app_update_available", "Update", "NVIDIA RTX news")).toBeNull();
    expect(resolve("scan_failed", "Scan failed", "NVIDIA driver error")).toBeNull();
    expect(resolve("catalog_refresh_failed", "Refresh failed", "RTX manifest unreachable")).toBeNull();
  });
});

describe("vendorKeyForNotification — text fallback for un-stamped vendor-bound kinds", () => {
  it("maps GPU feature tokens to their vendor (word-boundary safe)", () => {
    expect(resolve("apply_success", "Updated game", "DLSS Frame Generation")).toBe("nvidia");
    expect(resolve("catalog_update_available", "FSR 3.1 available")).toBe("amd");
    expect(resolve("catalog_update_available", "XeSS 2.0 available")).toBe("intel");
    expect(resolve("catalog_update_available", "DirectStorage 1.2")).toBe("microsoft");
  });

  it("maps vendor names and GPU brands via the shared brand resolver", () => {
    expect(resolve("driver_update_available", "GPU driver 999 — NVIDIA GeForce RTX 4070")).toBe("nvidia");
    expect(resolve("driver_update_available", "AMD Radeon RX 7900 driver")).toBe("amd");
    expect(resolve("driver_update_available", "Intel Arc A770 driver")).toBe("intel");
  });

  it("returns null when no vendor signal is present", () => {
    expect(resolve("dll_updates_available", "8 updates ready in 1 game", "Open the Library")).toBeNull();
    expect(resolve("apply_success", "Updated game", "done")).toBeNull();
  });
});
