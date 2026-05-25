import { describe, it, expect } from "vitest";
import { diffCatalogLatest } from "@/lib/stores";

describe("diffCatalogLatest", () => {
  it("first load (empty before) emits nothing — avoids false 'new version' spam", () => {
    const after = { dlss_sr: "2.0.0.0", fsr_upscaler: "3.1.0" };
    expect(diffCatalogLatest({}, after)).toEqual([]);
  });

  it("reports only families whose version actually changed", () => {
    const before = { dlss_sr: "1.0.0.0", fsr_upscaler: "3.1.0", xess_sr: "1.2.0" };
    const after = { dlss_sr: "2.0.0.0", fsr_upscaler: "3.1.0", xess_sr: "1.3.0" };
    const diffs = diffCatalogLatest(before, after).sort((a, b) => a.family.localeCompare(b.family));
    expect(diffs).toEqual([
      { family: "dlss_sr", oldVersion: "1.0.0.0", newVersion: "2.0.0.0" },
      { family: "xess_sr", oldVersion: "1.2.0", newVersion: "1.3.0" },
    ]);
  });

  it("a brand-new family with no prior version is skipped (old version is falsy)", () => {
    const before = { dlss_sr: "1.0.0.0" };
    const after = { dlss_sr: "1.0.0.0", xess_fg: "1.0.0" };
    expect(diffCatalogLatest(before, after)).toEqual([]);
  });

  it("no changes yields an empty diff", () => {
    const same = { dlss_sr: "2.0.0.0" };
    expect(diffCatalogLatest(same, { ...same })).toEqual([]);
  });

  it("a family removed from 'after' is not reported (only iterates after)", () => {
    const before = { dlss_sr: "1.0.0.0", reflex: "2.0.0.0" };
    const after = { dlss_sr: "1.0.0.0" };
    expect(diffCatalogLatest(before, after)).toEqual([]);
  });
});
