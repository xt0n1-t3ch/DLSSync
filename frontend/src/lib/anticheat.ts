import { get } from "svelte/store";
import type { AntiCheatReport, DetectedAntiCheat } from "./api";
import { translate, locale } from "./i18n/index";

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
  return translate(get(locale), "anticheat.linuxStatus", { status: report.status });
}

export function warningMessage(report: AntiCheatReport): string {
  const names = detectedNames(report);
  const banRisk = hasAnyKind(report, "anti_cheat");
  const tamperRisk = hasAnyKind(report, "anti_tamper");
  const loc = get(locale);

  if (banRisk) {
    const base = translate(loc, "anticheat.banRisk", { names });
    const tamperSuffix = tamperRisk ? translate(loc, "anticheat.banRiskTamperSuffix") : "";
    return base + tamperSuffix + translate(loc, "anticheat.banRiskClose");
  }
  if (tamperRisk) {
    return translate(loc, "anticheat.tamperRisk", { names });
  }
  return translate(loc, "anticheat.drm", { names });
}
