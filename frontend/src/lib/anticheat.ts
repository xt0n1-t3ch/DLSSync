import type { AntiCheatReport, DetectedAntiCheat } from "./api";

export type ProtectionSeverity = "danger" | "warning";

export function hasAntiCheat(report: AntiCheatReport | null | undefined): report is AntiCheatReport {
  return !!report && report.detected.length > 0;
}

export function detectedNames(report: AntiCheatReport): string {
  return report.detected.map((d) => d.anticheat).join(", ");
}

/** Any kernel/usermode anti-cheat carries account-ban risk → danger; otherwise
 * the detections are anti-tamper / DRM only → launch-fail risk → warning. */
export function severity(report: AntiCheatReport): ProtectionSeverity {
  return report.detected.some((d) => d.kind === "anti_cheat") ? "danger" : "warning";
}

export function hasAnyKind(report: AntiCheatReport, kind: DetectedAntiCheat["kind"]): boolean {
  return report.detected.some((d) => d.kind === kind);
}

export function statusNote(report: AntiCheatReport): string | null {
  if (!report.status) return null;
  if (report.status === "Anti-Tamper") return null;
  return `Linux / Wine compatibility: ${report.status}.`;
}

export function warningMessage(report: AntiCheatReport): string {
  const names = detectedNames(report);
  const banRisk = hasAnyKind(report, "anti_cheat");
  const tamperRisk = hasAnyKind(report, "anti_tamper");

  if (banRisk) {
    const tamperTail = tamperRisk
      ? " The anti-tamper layer may additionally refuse to launch on a signature mismatch."
      : "";
    return (
      `${names} detected. Swapping a DLL or forcing a DLSS driver-profile override into an ` +
      `anti-cheat title can be read as a tampered file and may get your account kicked or ` +
      `banned.${tamperTail} Check the game's policy before applying.`
    );
  }
  if (tamperRisk) {
    return (
      `${names} detected. Swapping a DLL or forcing a DLSS preset override may fail the ` +
      `game's tamper-protection signature check and prevent it from launching. Backups in ` +
      `DLSSync restore the originals instantly if that happens.`
    );
  }
  return (
    `${names} detected. This game's store DRM validates its files; a DLL swap is usually fine ` +
    `but can require a launcher integrity re-check. Backups in DLSSync restore the originals.`
  );
}
