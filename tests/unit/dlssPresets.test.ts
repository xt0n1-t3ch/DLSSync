import { describe, it, expect } from "vitest";
import type { DlssPreset } from "@/lib/api";
import {
  SR_PRESET_OPTIONS,
  FG_MODE_OPTIONS,
  FG_COUNT_OPTIONS,
  emptyDlssConfig,
  presetLabel,
  dlss4Available,
  dynamicMfgAvailable,
  hasActiveOverride,
  DLSS4_MIN_DRIVER_PACKED,
  DYNAMIC_MFG_MIN_DRIVER_PACKED,
} from "@/lib/dlss";

describe("dlss override option tables", () => {
  it("exposes super-resolution, frame-gen mode and count options", () => {
    expect(SR_PRESET_OPTIONS.some((o) => o.value === "recommended")).toBe(true);
    expect(SR_PRESET_OPTIONS.some((o) => o.value === "k")).toBe(true);
    expect(FG_MODE_OPTIONS.map((o) => o.value)).toEqual(["app_controlled", "fixed", "dynamic"]);
    expect(FG_COUNT_OPTIONS.map((o) => o.value)).toEqual(["app_controlled", "x2", "x3", "x4"]);
  });

  it("labels the full A-O preset range so any externally-set preset is shown", () => {
    expect(presetLabel("k")).toContain("Preset K");
    expect(presetLabel("recommended")).toContain("Recommended");
    expect(presetLabel("a")).toContain("Preset A");
    expect(presetLabel("m")).toContain("Preset M");
    expect(presetLabel("o")).toContain("Preset O");
    for (const letter of ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o"] as DlssPreset[]) {
      expect(SR_PRESET_OPTIONS.some((o) => o.value === letter)).toBe(true);
    }
  });

  it("falls back to the upper-cased value for an unrecognized preset", () => {
    expect(presetLabel("z" as DlssPreset)).toBe("Z");
  });
});

describe("driver-version gating", () => {
  it("requires 572.16 for DLSS 4 overrides", () => {
    expect(dlss4Available(DLSS4_MIN_DRIVER_PACKED)).toBe(true);
    expect(dlss4Available(DLSS4_MIN_DRIVER_PACKED - 1)).toBe(false);
    expect(dlss4Available(59174)).toBe(true);
  });

  it("requires 595.97 for dynamic multi frame generation", () => {
    expect(DYNAMIC_MFG_MIN_DRIVER_PACKED).toBe(59597);
    expect(dynamicMfgAvailable(DYNAMIC_MFG_MIN_DRIVER_PACKED)).toBe(true);
    expect(dynamicMfgAvailable(DYNAMIC_MFG_MIN_DRIVER_PACKED - 1)).toBe(false);
    expect(dynamicMfgAvailable(57216)).toBe(false);
  });
});

describe("every option is self-explanatory with a source link", () => {
  it.each([
    ["SR presets", SR_PRESET_OPTIONS],
    ["FG modes", FG_MODE_OPTIONS],
    ["FG counts", FG_COUNT_OPTIONS],
  ])("%s carry a label, description and https source URL", (_name, options) => {
    for (const option of options) {
      expect(option.label.length).toBeGreaterThan(0);
      expect(option.description.length).toBeGreaterThan(12);
      expect(option.sourceUrl).toMatch(/^https:\/\//);
    }
  });
});

describe("config helpers", () => {
  it("empty config has no active override", () => {
    expect(hasActiveOverride(emptyDlssConfig())).toBe(false);
  });

  it("any set field marks the config active", () => {
    expect(hasActiveOverride({ ...emptyDlssConfig(), enable_sr_dll_override: true })).toBe(true);
    expect(hasActiveOverride({ ...emptyDlssConfig(), fg_mode: "dynamic" })).toBe(true);
    expect(hasActiveOverride({ ...emptyDlssConfig(), fg_dynamic_target_fps: 240 })).toBe(true);
  });
});
