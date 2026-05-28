import { describe, it, expect } from "vitest";
import type { DriverStatusReport, GpuVendor, DriverUpdateStatus } from "@/lib/api";
import { canInstall, isOpenPageOnly, driverPageUrl, vendorHelpUrl } from "@/lib/drivers";

function report(
  vendor: GpuVendor,
  status: DriverUpdateStatus,
  opts: { download?: string; notes?: string | null; notesPdf?: string | null } = {},
): DriverStatusReport {
  const hasLatest = status === "update_available" || status === "up_to_date";
  return {
    device: { class: "gpu", vendor, pci_vendor_id: 0, pci_device_id: 0, model: "GPU" },
    installed: { packed: 1, display: "1.0", raw: "1.0" },
    latest: hasLatest
      ? {
          vendor,
          version: { packed: 2, display: "2.0", raw: "2.0" },
          channel: "stable",
          display_version: null,
          is_beta: false,
          download_url: opts.download ?? "",
          size_bytes: 0,
          signature_subject: "x",
          released_at: null,
          release_notes_url: opts.notes === undefined ? "https://vendor/notes" : opts.notes,
          changelog: opts.notesPdf
            ? { highlights: [], fixed: [], notes_page_url: opts.notesPdf }
            : null,
        }
      : null,
    status,
  };
}

describe("driver action routing", () => {
  it("canInstall is true only for an available update with a direct download URL", () => {
    expect(canInstall(report("nvidia", "update_available", { download: "https://nv/d.exe" }))).toBe(true);
    expect(canInstall(report("intel", "update_available", { download: "https://intel/g.exe" }))).toBe(true);
    expect(canInstall(report("amd", "update_available", { download: "" }))).toBe(false);
    expect(canInstall(report("nvidia", "up_to_date", { download: "https://nv/d.exe" }))).toBe(false);
  });

  it("isOpenPageOnly is true for AMD-style updates: available, no download, has a page", () => {
    expect(isOpenPageOnly(report("amd", "update_available", { download: "", notes: "https://amd/rn" }))).toBe(true);
    expect(isOpenPageOnly(report("amd", "update_available", { download: "", notes: null }))).toBe(false);
    expect(isOpenPageOnly(report("nvidia", "update_available", { download: "https://nv/d.exe" }))).toBe(false);
    expect(isOpenPageOnly(report("intel", "up_to_date"))).toBe(false);
  });

  it("canInstall and isOpenPageOnly are mutually exclusive", () => {
    const nv = report("nvidia", "update_available", { download: "https://nv/d.exe" });
    const amd = report("amd", "update_available", { download: "", notes: "https://amd/rn" });
    expect(canInstall(nv) && isOpenPageOnly(nv)).toBe(false);
    expect(canInstall(amd) && isOpenPageOnly(amd)).toBe(false);
  });

  it("driverPageUrl prefers release notes, falls back to the changelog PDF, else null", () => {
    expect(driverPageUrl(report("nvidia", "update_available", { download: "x", notes: "https://nv/rn" }))).toBe(
      "https://nv/rn",
    );
    expect(
      driverPageUrl(report("nvidia", "update_available", { download: "x", notes: null, notesPdf: "https://nv/rn.pdf" })),
    ).toBe("https://nv/rn.pdf");
    expect(driverPageUrl(report("nvidia", "update_available", { download: "x", notes: null }))).toBeNull();
    expect(driverPageUrl(report("intel", "unknown"))).toBeNull();
  });

  it("vendorHelpUrl routes each vendor to its official finder, with a Windows fallback", () => {
    expect(vendorHelpUrl("intel")).toContain("intel.com");
    expect(vendorHelpUrl("nvidia")).toContain("nvidia.com");
    expect(vendorHelpUrl("amd")).toContain("amd.com");
    expect(vendorHelpUrl("other")).toContain("microsoft.com");
  });
});
