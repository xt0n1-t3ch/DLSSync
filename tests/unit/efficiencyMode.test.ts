import { describe, it, expect, beforeEach } from "vitest";
import {
  readEfficiencyPreference,
  writeEfficiencyPreference,
  EFFICIENCY_PREF_KEY,
  LEGACY_EFFICIENCY_PREF_KEY,
} from "@/lib/efficiencyMode";

describe("readEfficiencyPreference", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns false on first run when both keys are absent", () => {
    expect(localStorage.getItem(EFFICIENCY_PREF_KEY)).toBeNull();
    expect(localStorage.getItem(LEGACY_EFFICIENCY_PREF_KEY)).toBeNull();
    expect(readEfficiencyPreference()).toBe(false);
  });

  it("prioritizes current key over legacy key", () => {
    localStorage.setItem(EFFICIENCY_PREF_KEY, "true");
    localStorage.setItem(LEGACY_EFFICIENCY_PREF_KEY, "false");
    expect(readEfficiencyPreference()).toBe(true);
  });

  it("uses legacy key when current key is absent", () => {
    localStorage.setItem(LEGACY_EFFICIENCY_PREF_KEY, "true");
    expect(readEfficiencyPreference()).toBe(true);
  });

  it("returns false for legacy key when it is not 'true'", () => {
    localStorage.setItem(LEGACY_EFFICIENCY_PREF_KEY, "false");
    expect(readEfficiencyPreference()).toBe(false);
  });

  it("returns false for legacy key when it is any other value", () => {
    localStorage.setItem(LEGACY_EFFICIENCY_PREF_KEY, "anything");
    expect(readEfficiencyPreference()).toBe(false);
  });

  it("parses current key as true when it is not 'false'", () => {
    localStorage.setItem(EFFICIENCY_PREF_KEY, "true");
    expect(readEfficiencyPreference()).toBe(true);
  });

  it("parses current key as false when it is exactly 'false'", () => {
    localStorage.setItem(EFFICIENCY_PREF_KEY, "false");
    expect(readEfficiencyPreference()).toBe(false);
  });

  it("parses current key as true for any value other than 'false'", () => {
    localStorage.setItem(EFFICIENCY_PREF_KEY, "1");
    expect(readEfficiencyPreference()).toBe(true);

    localStorage.clear();
    localStorage.setItem(EFFICIENCY_PREF_KEY, "yes");
    expect(readEfficiencyPreference()).toBe(true);
  });
});

describe("writeEfficiencyPreference", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("stores the enabled boolean as a string in localStorage", () => {
    writeEfficiencyPreference(true);
    expect(localStorage.getItem(EFFICIENCY_PREF_KEY)).toBe("true");

    writeEfficiencyPreference(false);
    expect(localStorage.getItem(EFFICIENCY_PREF_KEY)).toBe("false");
  });

  it("dispatches a custom event with the enabled value", () => {
    const listener = (evt: CustomEvent<{ enabled: boolean }>) => {
      expect(evt.detail.enabled).toBe(true);
    };

    window.addEventListener("dlssync:efficiency-mode", listener as EventListener);
    writeEfficiencyPreference(true);
    window.removeEventListener("dlssync:efficiency-mode", listener as EventListener);
  });
});

describe("readEfficiencyPreference round-trip", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("round-trip: write true, then read true", () => {
    writeEfficiencyPreference(true);
    expect(readEfficiencyPreference()).toBe(true);
  });

  it("round-trip: write false, then read false", () => {
    writeEfficiencyPreference(false);
    expect(readEfficiencyPreference()).toBe(false);
  });
});
