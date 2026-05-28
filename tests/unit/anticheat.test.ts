import { describe, it, expect } from "vitest";
import {
  hasAntiCheat,
  detectedNames,
  statusNote,
  warningMessage,
  severity,
  hasAnyKind,
} from "@/lib/anticheat";
import type { AntiCheatReport } from "@/lib/api";

const kernelAc: AntiCheatReport = {
  detected: [
    { anticheat: "Easy Anti-Cheat", kind: "anti_cheat", source: "binary" },
    { anticheat: "BattlEye", kind: "anti_cheat", source: "dataset" },
  ],
  status: "Broken",
  source_url: "https://www.pcgamingwiki.com/wiki/Glossary:Anti-cheat",
};

const denuvoOnly: AntiCheatReport = {
  detected: [
    { anticheat: "Denuvo Anti-Tamper", kind: "anti_tamper", source: "pe" },
    { anticheat: "Ubisoft Connect DRM", kind: "drm", source: "dataset" },
  ],
  status: "Anti-Tamper",
  source_url: "https://www.pcgamingwiki.com/wiki/Glossary:Digital_rights_management",
};

const clean: AntiCheatReport = { detected: [], status: null, source_url: null };

describe("anti-cheat warning helpers", () => {
  it("flags a report with detections and ignores a clean one", () => {
    expect(hasAntiCheat(kernelAc)).toBe(true);
    expect(hasAntiCheat(clean)).toBe(false);
    expect(hasAntiCheat(null)).toBe(false);
    expect(hasAntiCheat(undefined)).toBe(false);
  });

  it("joins detected names", () => {
    expect(detectedNames(kernelAc)).toBe("Easy Anti-Cheat, BattlEye");
    expect(detectedNames(clean)).toBe("");
  });

  it("derives severity from kind: kernel anti-cheat is danger, tamper-only is warning", () => {
    expect(severity(kernelAc)).toBe("danger");
    expect(severity(denuvoOnly)).toBe("warning");
  });

  it("reports presence of a given kind", () => {
    expect(hasAnyKind(kernelAc, "anti_cheat")).toBe(true);
    expect(hasAnyKind(kernelAc, "anti_tamper")).toBe(false);
    expect(hasAnyKind(denuvoOnly, "anti_tamper")).toBe(true);
    expect(hasAnyKind(denuvoOnly, "drm")).toBe(true);
  });

  it("renders the dataset status note only for Linux/Wine statuses", () => {
    expect(statusNote(kernelAc)).toContain("Broken");
    expect(statusNote(denuvoOnly)).toBeNull();
    expect(statusNote(clean)).toBeNull();
  });

  it("warns about ban risk for kernel anti-cheat and names the engines", () => {
    const msg = warningMessage(kernelAc);
    expect(msg).toContain("Easy Anti-Cheat, BattlEye");
    expect(msg.toLowerCase()).toContain("banned");
    expect(msg.toLowerCase()).toContain("dlss");
  });

  it("pivots to launch-fail copy when only anti-tamper / DRM is detected", () => {
    const msg = warningMessage(denuvoOnly);
    expect(msg).toContain("Denuvo Anti-Tamper, Ubisoft Connect DRM");
    expect(msg.toLowerCase()).toContain("tamper-protection");
    expect(msg.toLowerCase()).toContain("launching");
    expect(msg.toLowerCase()).not.toContain("banned");
  });

  it("layers ban + tamper copy when both kinds detected", () => {
    const mixed: AntiCheatReport = {
      detected: [
        { anticheat: "Easy Anti-Cheat", kind: "anti_cheat", source: "dataset" },
        { anticheat: "Denuvo Anti-Tamper", kind: "anti_tamper", source: "pe" },
      ],
      status: "Denied",
      source_url: null,
    };
    const msg = warningMessage(mixed).toLowerCase();
    expect(msg).toContain("banned");
    expect(msg).toContain("anti-tamper");
  });
});
