import { describe, it, expect } from "vitest";
import type { DllRecord } from "@/lib/api";
import {
  catalogShaKey,
  buildShasByVendor,
  targetVersion,
  dllRelation,
  isOutdated,
  gameStatusFromRecords,
  type RelationContext,
} from "@/lib/relation";

function rec(partial: Partial<DllRecord>): DllRecord {
  return {
    family: "dlss_sr",
    path: "C:\\Games\\x\\nvngx_dlss.dll",
    current_version: "1.0.0.0",
    sha256: null,
    ...partial,
  } as DllRecord;
}

function ctx(over: Partial<RelationContext> = {}): RelationContext {
  return { latestByKey: {}, shas: {}, shasByVendor: {}, ...over };
}

describe("catalogShaKey", () => {
  it("lowercases and joins vendor::family::filename", () => {
    expect(catalogShaKey("NVIDIA", "DLSS_SR", "NvNgx_Dlss.dll")).toBe("nvidia::dlss_sr::nvngx_dlss.dll");
  });
});

describe("buildShasByVendor", () => {
  it("groups lowercased shas by vendor prefix", () => {
    const map = buildShasByVendor({
      "nvidia::dlss_sr::a.dll": "ABC",
      "nvidia::dlss_fg::b.dll": "DEF",
      "amd::fsr::c.dll": "GHI",
    });
    expect(map.nvidia.has("abc")).toBe(true);
    expect(map.nvidia.has("def")).toBe(true);
    expect(map.amd.has("ghi")).toBe(true);
    expect(map.nvidia.size).toBe(2);
  });

  it("empty input yields empty map", () => {
    expect(Object.keys(buildShasByVendor({})).length).toBe(0);
  });
});

describe("targetVersion", () => {
  it("pinned version wins over catalog", () => {
    expect(targetVersion(rec({}), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }), "9.9.9")).toBe("9.9.9");
  });

  it("falls back to catalog latest by family key", () => {
    expect(targetVersion(rec({}), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe("2.0.0.0");
  });

  it("returns null when no catalog entry", () => {
    expect(targetVersion(rec({}), ctx())).toBeNull();
  });
});

describe("dllRelation", () => {
  it("no-target when catalog lacks the family", () => {
    expect(dllRelation(rec({ current_version: "1.0.0.0" }), ctx())).toBe("no-target");
  });

  it("outdated when current < target", () => {
    expect(dllRelation(rec({ current_version: "1.0.0.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe("outdated");
  });

  it("ahead when current > target", () => {
    expect(dllRelation(rec({ current_version: "3.1.0.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe("ahead");
  });

  it("same when versions are equal", () => {
    expect(dllRelation(rec({ current_version: "2.0.0.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe("same");
  });

  it("uneven version segments compare numerically (2.0 vs 2.0.0.1)", () => {
    expect(dllRelation(rec({ current_version: "2.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.1" } }))).toBe("outdated");
    expect(dllRelation(rec({ current_version: "2.0.0.1" }), ctx({ latestByKey: { dlss_sr: "2.0" } }))).toBe("ahead");
  });

  it("exact sha match short-circuits to same even if version is lower", () => {
    const r = rec({ current_version: "1.0.0.0", sha256: "DEAD" });
    const c = ctx({
      latestByKey: { dlss_sr: "2.0.0.0" },
      shas: { "nvidia::dlss_sr::nvngx_dlss.dll": "dead" },
    });
    expect(dllRelation(r, c)).toBe("same");
  });

  it("known vendor sha (any file) short-circuits to same", () => {
    const r = rec({ current_version: "1.0.0.0", sha256: "BEEF" });
    const c = ctx({
      latestByKey: { dlss_sr: "2.0.0.0" },
      shasByVendor: { nvidia: new Set(["beef"]) },
    });
    expect(dllRelation(r, c)).toBe("same");
  });

  it("no current_version with a target and no sha is no-target", () => {
    const r = rec({ current_version: null as unknown as string });
    expect(dllRelation(r, ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe("no-target");
  });
});

describe("isOutdated", () => {
  it("mirrors dllRelation === outdated", () => {
    expect(isOutdated(rec({ current_version: "1.0.0.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe(true);
    expect(isOutdated(rec({ current_version: "2.0.0.0" }), ctx({ latestByKey: { dlss_sr: "2.0.0.0" } }))).toBe(false);
  });
});

describe("gameStatusFromRecords", () => {
  const outdatedCtx = ctx({ latestByKey: { dlss_sr: "2.0.0.0" } });

  it("scan error takes precedence over everything", () => {
    expect(gameStatusFromRecords([rec({})], outdatedCtx, [], "boom")).toBe("scan_failed");
  });

  it("undefined records is unknown", () => {
    expect(gameStatusFromRecords(undefined, outdatedCtx)).toBe("unknown");
  });

  it("empty records is no_dlls", () => {
    expect(gameStatusFromRecords([], outdatedCtx)).toBe("no_dlls");
  });

  it("any outdated record makes the game outdated", () => {
    const records = [rec({ current_version: "2.0.0.0" }), rec({ current_version: "1.0.0.0" })];
    expect(gameStatusFromRecords(records, outdatedCtx)).toBe("outdated");
  });

  it("disabled families are skipped from the outdated check", () => {
    const records = [rec({ family: "dlss_sr", current_version: "1.0.0.0" })];
    expect(gameStatusFromRecords(records, outdatedCtx, ["dlss_sr"])).toBe("up_to_date");
  });

  it("all current is up_to_date", () => {
    expect(gameStatusFromRecords([rec({ current_version: "2.0.0.0" })], outdatedCtx)).toBe("up_to_date");
  });
});
