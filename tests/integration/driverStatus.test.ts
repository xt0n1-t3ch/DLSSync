import { describe, it, expect } from "vitest";
import type { DriverStatusReport, DriverUpdateStatus } from "@/lib/api";
import {
  driverStatusLabel,
  driverStatusTone,
  hasDriverUpdate,
  driverUpdateCount,
  sortDriverReports,
} from "@/lib/drivers";

function makeReport(model: string, status: DriverUpdateStatus): DriverStatusReport {
  return {
    device: { class: "gpu", vendor: "nvidia", pci_vendor_id: 0x10de, pci_device_id: 0, model },
    installed: { packed: 59174, display: "591.74", raw: "32.0.15.9174" },
    latest:
      status === "update_available"
        ? {
            vendor: "nvidia",
            version: { packed: 61047, display: "610.47", raw: "610.47" },
            channel: "stable",
            display_version: null,
            is_beta: false,
            download_url: "https://us.download.nvidia.com/Windows/610.47/610.47.exe",
            size_bytes: 1024 * 1024 * 978,
            signature_subject: "NVIDIA Corporation",
            released_at: null,
            release_notes_url: null,
            changelog: null,
          }
        : null,
    status,
  };
}

describe("driver status labels and tones", () => {
  it("labels every status with a human string", () => {
    expect(driverStatusLabel("update_available")).toBe("Update available");
    expect(driverStatusLabel("up_to_date")).toBe("Up to date");
    expect(driverStatusLabel("unknown")).toBe("Unknown");
    expect(driverStatusLabel("unsupported")).toBe("Not supported");
  });

  it("maps every status to a tone", () => {
    expect(driverStatusTone("update_available")).toBe("accent");
    expect(driverStatusTone("up_to_date")).toBe("success");
    expect(driverStatusTone("unknown")).toBe("warning");
    expect(driverStatusTone("unsupported")).toBe("muted");
  });
});

describe("driver update detection", () => {
  it("flags only update_available as an update", () => {
    expect(hasDriverUpdate(makeReport("a", "update_available"))).toBe(true);
    expect(hasDriverUpdate(makeReport("a", "up_to_date"))).toBe(false);
    expect(hasDriverUpdate(makeReport("a", "unknown"))).toBe(false);
  });

  it("counts only reports with an available update", () => {
    const reports = [
      makeReport("a", "update_available"),
      makeReport("b", "up_to_date"),
      makeReport("c", "update_available"),
      makeReport("d", "unsupported"),
    ];
    expect(driverUpdateCount(reports)).toBe(2);
  });
});

describe("sortDriverReports", () => {
  it("orders update_available first, then unknown, up_to_date, unsupported", () => {
    const reports = [
      makeReport("z-uptodate", "up_to_date"),
      makeReport("m-unsupported", "unsupported"),
      makeReport("a-update", "update_available"),
      makeReport("b-unknown", "unknown"),
    ];
    expect(sortDriverReports(reports).map((r) => r.status)).toEqual([
      "update_available",
      "unknown",
      "up_to_date",
      "unsupported",
    ]);
  });

  it("breaks ties by model name and does not mutate the input", () => {
    const reports = [
      makeReport("Radeon", "update_available"),
      makeReport("Arc", "update_available"),
    ];
    const sorted = sortDriverReports(reports);
    expect(sorted.map((r) => r.device.model)).toEqual(["Arc", "Radeon"]);
    expect(reports[0].device.model).toBe("Radeon");
  });
});
