import type { DriverStatusReport, DriverUpdateStatus } from "./api";

export type DriverTone = "success" | "accent" | "muted" | "warning";

const STATUS_LABEL: Record<DriverUpdateStatus, string> = {
  update_available: "Update available",
  up_to_date: "Up to date",
  unknown: "Unknown",
  unsupported: "Not supported",
};

const STATUS_TONE: Record<DriverUpdateStatus, DriverTone> = {
  update_available: "accent",
  up_to_date: "success",
  unknown: "warning",
  unsupported: "muted",
};

const STATUS_RANK: Record<DriverUpdateStatus, number> = {
  update_available: 0,
  unknown: 1,
  up_to_date: 2,
  unsupported: 3,
};

export function driverStatusLabel(status: DriverUpdateStatus): string {
  return STATUS_LABEL[status] ?? STATUS_LABEL.unknown;
}

export function driverStatusTone(status: DriverUpdateStatus): DriverTone {
  return STATUS_TONE[status] ?? "muted";
}

export function hasDriverUpdate(report: DriverStatusReport): boolean {
  return report.status === "update_available";
}

/** Page to open for a driver: the device's own release-notes page, falling back
 *  to a changelog notes PDF. Used both for "Release notes" and AMD's
 *  "Open download page" action. */
export function driverPageUrl(report: DriverStatusReport): string | null {
  return report.latest?.release_notes_url ?? report.latest?.changelog?.notes_page_url ?? null;
}

/** An update is in-app installable only when the vendor gives a direct download
 *  URL (NVIDIA, Intel). */
export function canInstall(report: DriverStatusReport): boolean {
  return report.status === "update_available" && !!report.latest?.download_url;
}

/** An update exists but there is no safe direct download (AMD: the real `.exe`
 *  is EULA-gated and its filename is unstable) — route the user to the official
 *  page instead of fabricating a link. */
export function isOpenPageOnly(report: DriverStatusReport): boolean {
  return (
    report.status === "update_available" &&
    !report.latest?.download_url &&
    !!driverPageUrl(report)
  );
}

const VENDOR_HELP_URL: Record<string, string> = {
  intel: "https://www.intel.com/content/www/us/en/support/detect.html",
  nvidia: "https://www.nvidia.com/Download/index.aspx",
  amd: "https://www.amd.com/en/support/download/drivers.html",
};

/** Official "find my driver" page for a vendor — shown when status is unknown
 *  (e.g. an OEM-locked integrated GPU) so the user is never left at a dead end. */
export function vendorHelpUrl(vendor: string): string {
  return (
    VENDOR_HELP_URL[vendor] ??
    "https://support.microsoft.com/windows/update-drivers-in-windows-ec62f46c-ff14-c91d-eead-d7126dc1f7b6"
  );
}

export function driverUpdateCount(reports: DriverStatusReport[]): number {
  return reports.reduce((total, report) => (hasDriverUpdate(report) ? total + 1 : total), 0);
}

export function sortDriverReports(reports: DriverStatusReport[]): DriverStatusReport[] {
  return [...reports].sort((a, b) => {
    const rank = (STATUS_RANK[a.status] ?? 9) - (STATUS_RANK[b.status] ?? 9);
    return rank !== 0 ? rank : a.device.model.localeCompare(b.device.model);
  });
}
