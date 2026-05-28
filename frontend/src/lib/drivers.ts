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

export function driverUpdateCount(reports: DriverStatusReport[]): number {
  return reports.reduce((total, report) => (hasDriverUpdate(report) ? total + 1 : total), 0);
}

export function sortDriverReports(reports: DriverStatusReport[]): DriverStatusReport[] {
  return [...reports].sort((a, b) => {
    const rank = (STATUS_RANK[a.status] ?? 9) - (STATUS_RANK[b.status] ?? 9);
    return rank !== 0 ? rank : a.device.model.localeCompare(b.device.model);
  });
}
