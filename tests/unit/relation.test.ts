import { describe, it, expect } from "vitest";
import type { DllRecord, UpdatePreferences } from "@/lib/api";
import {
  catalogShaKey,
  buildShasByVendor,
  targetVersion,
  dllRelation,
  isOutdated,
  isStreamlinePlugin,
  recordUpdatable,
  gameStatusFromRecords,
  versionMajor,
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

function prefs(over: Partial<UpdatePreferences> = {}): UpdatePreferences {
  return {
    update_dlss: true,
    update_dlss_fg: true,
    update_dlss_rr: true,
    update_streamline: false,
    update_reflex: true,
    update_xess: true,
    update_fsr: true,
    update_direct_storage: true,
    create_backups: true,
    auto_apply_all_on_rescan: false,
    ...over,
  };
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

  it("a Streamline plugin outdated only is up_to_date when the Streamline switch is off", () => {
    const records = [rec({ family: "dlss_sr", path: "C:\\g\\sl.dlss.dll", current_version: "2.0.0.0" })];
    const slCtx = ctx({ latestByKey: { dlss_sr: "2.5.0.0" } });
    expect(gameStatusFromRecords(records, slCtx, [], null, prefs({ update_streamline: false }))).toBe("up_to_date");
    expect(gameStatusFromRecords(records, slCtx, [], null, prefs({ update_streamline: true }))).toBe("outdated");
  });

  it("a same-major Streamline plugin is offered even when a DLSS Enabler is present", () => {
    const records = [rec({ family: "reflex", path: "C:\\g\\sl.reflex.dll", current_version: "2.0.0.0" })];
    const reflexCtx = ctx({ latestByKey: { reflex: "2.5.0.0" } });
    expect(gameStatusFromRecords(records, reflexCtx, [], null, prefs({ update_streamline: true }))).toBe("outdated");
    expect(gameStatusFromRecords(records, reflexCtx, [], null, prefs({ update_streamline: false }))).toBe("up_to_date");
  });
});

describe("isStreamlinePlugin", () => {
  it("matches sl.* dlls case-insensitively and rejects NGX/other dlls", () => {
    expect(isStreamlinePlugin("sl.dlss.dll")).toBe(true);
    expect(isStreamlinePlugin("SL.Reflex.DLL")).toBe(true);
    expect(isStreamlinePlugin("sl.interposer.dll")).toBe(true);
    expect(isStreamlinePlugin("nvngx_dlss.dll")).toBe(false);
    expect(isStreamlinePlugin("libxess.dll")).toBe(false);
  });
});

describe("sl_dlss feature plugins (v1.5.3 source-less, never mis-offered)", () => {
  it("sl.dlss_g.dll is never offered the nvngx 310.x version (the v1.5.2 Nexus bug)", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.7.30.0" });
    const c = ctx({ latestByKey: { dlss_fg: "310.6.0.0" } });
    expect(targetVersion(slFg, c)).toBeNull();
    expect(dllRelation(slFg, c)).toBe("no-target");
    expect(isOutdated(slFg, c)).toBe(false);
    expect(gameStatusFromRecords([slFg], c, [], null, prefs({ update_streamline: true }))).toBe("up_to_date");
  });

  it("sl.dlss.dll and sl.dlss_d.dll are likewise never offered their nvngx versions", () => {
    const slSr = rec({ family: "sl_dlss_sr", path: "C:\\g\\sl.dlss.dll", current_version: "2.7.30.0" });
    const slRr = rec({ family: "sl_dlss_rr", path: "C:\\g\\sl.dlss_d.dll", current_version: "2.7.30.0" });
    const c = ctx({ latestByKey: { dlss_sr: "310.6.0.0", dlss_rr: "310.6.0.0" } });
    expect(dllRelation(slSr, c)).toBe("no-target");
    expect(dllRelation(slRr, c)).toBe("no-target");
  });
});

describe("sl_dlss feature plugins (v1.6 — sourced from the official SDK, offers light up)", () => {
  it("offers the matched 2.x version once the catalog sources sl_dlss_fg", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.10.3.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", dlss_fg: "310.6.0.0" } });
    expect(targetVersion(slFg, c)).toBe("2.11.1.0");
    expect(dllRelation(slFg, c)).toBe("outdated");
    expect(recordUpdatable(slFg, prefs({ update_streamline: true }))).toBe(true);
    expect(gameStatusFromRecords([slFg], c, [], null, prefs({ update_streamline: true }))).toBe("outdated");
  });

  it("targets the 2.x sl scheme, never the 310.x nvngx scheme that shares the SDK release", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.10.3.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", dlss_fg: "310.6.0.0" } });
    expect(targetVersion(slFg, c)).toBe("2.11.1.0");
  });

  it("stays gated behind the Streamline master switch", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.10.3.0" });
    expect(recordUpdatable(slFg, prefs({ update_streamline: false }))).toBe(false);
    expect(recordUpdatable(slFg, prefs({ update_streamline: true }))).toBe(true);
  });
});

describe("Nexus Subnautica 2 report (Kronprinz77, 2026-05-30) — end-to-end regression", () => {
  it("never offers sl.dlss_g.dll the 310.x nvngx version, only its own 2.x line", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\Subnautica 2\\sl.dlss_g.dll", current_version: "2.7.30.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", dlss_fg: "310.6.0.0" } });
    expect(targetVersion(slFg, c)).toBe("2.11.1.0");
    expect(targetVersion(slFg, c)).not.toBe("310.6.0.0");
  });

  it("offers the same-major Streamline set under a DLSS Enabler (keeps the Enabler working)", () => {
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", sl_dlss_sr: "2.11.1.0" } });
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\Subnautica 2\\sl.dlss_g.dll", current_version: "2.7.30.0" });
    const slSr = rec({ family: "sl_dlss_sr", path: "C:\\Subnautica 2\\sl.dlss.dll", current_version: "2.7.30.0" });
    expect(recordUpdatable(slFg, prefs({ update_streamline: true }))).toBe(true);
    expect(recordUpdatable(slSr, prefs({ update_streamline: true }))).toBe(true);
    expect(gameStatusFromRecords([slFg, slSr], c, [], null, prefs({ update_streamline: true }))).toBe("outdated");
    expect(gameStatusFromRecords([slFg, slSr], c, [], null, prefs({ update_streamline: false }))).toBe("up_to_date");
  });

  it("still offers the nvngx runtime regardless of the Streamline switch", () => {
    const ngx = rec({ family: "dlss_fg", path: "C:\\Subnautica 2\\nvngx_dlssg.dll", current_version: "310.1.0.0" });
    expect(recordUpdatable(ngx, prefs({ update_dlss_fg: true, update_streamline: false }))).toBe(true);
  });
});

describe("versionMajor", () => {
  it("reads the leading numeric segment, null on non-numeric/empty", () => {
    expect(versionMajor("310.6.0.0")).toBe(310);
    expect(versionMajor("2.11.1.0")).toBe(2);
    expect(versionMajor("2")).toBe(2);
    expect(versionMajor("")).toBeNull();
    expect(versionMajor("dev")).toBeNull();
  });
});

describe("scheme-aware Streamline offer (v1.6.1)", () => {
  it("leaves a 310.x-stamped sl.dlss_g alone against a 2.x catalog", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "310.6.0.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", dlss_fg: "310.6.0.0" } });
    expect(dllRelation(slFg, c)).toBe("same");
    expect(isOutdated(slFg, c)).toBe(false);
    expect(gameStatusFromRecords([slFg], c, [], null, prefs({ update_streamline: true }))).toBe("up_to_date");
  });

  it("still offers a 2.x sl.dlss_g older than the 2.x catalog (the Nexus 2.7.30 case)", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.7.30.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0", dlss_fg: "310.6.0.0" } });
    expect(dllRelation(slFg, c)).toBe("outdated");
    expect(gameStatusFromRecords([slFg], c, [], null, prefs({ update_streamline: true }))).toBe("outdated");
  });

  it("does not offer a 3.x catalog to a 2.x sl plug-in", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "2.11.1.0" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "3.0.0.0" } });
    expect(dllRelation(slFg, c)).toBe("same");
  });

  it("non-Streamline nvngx still compares versions normally", () => {
    const ngx = rec({ family: "dlss_sr", path: "C:\\g\\nvngx_dlss.dll", current_version: "310.5.0.0" });
    const c = ctx({ latestByKey: { dlss_sr: "310.6.0.0" } });
    expect(dllRelation(ngx, c)).toBe("outdated");
  });

  it("a garbage version on a Streamline plug-in is left alone, never offered", () => {
    const slFg = rec({ family: "sl_dlss_fg", path: "C:\\g\\sl.dlss_g.dll", current_version: "dev" });
    const c = ctx({ latestByKey: { sl_dlss_fg: "2.11.1.0" } });
    expect(dllRelation(slFg, c)).toBe("same");
  });
});

describe("recordUpdatable", () => {
  it("permissive when no prefs are supplied (legacy)", () => {
    expect(recordUpdatable(rec({ path: "C:\\g\\sl.dlss.dll" }))).toBe(true);
  });

  it("NGX DLLs follow only their feature pref, never the Streamline switch", () => {
    const ngx = rec({ family: "dlss_sr", path: "C:\\g\\nvngx_dlss.dll" });
    expect(recordUpdatable(ngx, prefs({ update_dlss: true, update_streamline: false }))).toBe(true);
    expect(recordUpdatable(ngx, prefs({ update_dlss: false }))).toBe(false);
  });

  it("Streamline plugins require BOTH the feature pref and the Streamline switch", () => {
    const slSr = rec({ family: "dlss_sr", path: "C:\\g\\sl.dlss.dll" });
    expect(recordUpdatable(slSr, prefs({ update_dlss: true, update_streamline: false }))).toBe(false);
    expect(recordUpdatable(slSr, prefs({ update_dlss: true, update_streamline: true }))).toBe(true);
    expect(recordUpdatable(slSr, prefs({ update_dlss: false, update_streamline: true }))).toBe(false);
  });

  it("a Streamline plugin is updatable when opted in — a DLSS Enabler no longer blocks the offer", () => {
    const slReflex = rec({ family: "reflex", path: "C:\\g\\sl.reflex.dll" });
    expect(recordUpdatable(slReflex, prefs({ update_reflex: true, update_streamline: true }))).toBe(true);
    expect(recordUpdatable(slReflex, prefs({ update_reflex: true, update_streamline: false }))).toBe(false);
  });

  it("DirectStorage follows its own pref and is never gated by the Streamline switch", () => {
    const ds = rec({ family: "direct_storage", path: "C:\\g\\dstorage.dll" });
    expect(recordUpdatable(ds, prefs({ update_direct_storage: true, update_streamline: false }))).toBe(true);
    expect(recordUpdatable(ds, prefs({ update_direct_storage: false }))).toBe(false);
  });

  it("XeSS and FSR follow their own prefs, unaffected by the Streamline switch", () => {
    const xess = rec({ family: "xess_sr", path: "C:\\g\\libxess.dll" });
    const fsr = rec({ family: "fsr_upscaler", path: "C:\\g\\amd_fidelityfx_dx12.dll" });
    expect(recordUpdatable(xess, prefs({ update_xess: true, update_streamline: false }))).toBe(true);
    expect(recordUpdatable(xess, prefs({ update_xess: false }))).toBe(false);
    expect(recordUpdatable(fsr, prefs({ update_fsr: true, update_streamline: false }))).toBe(true);
    expect(recordUpdatable(fsr, prefs({ update_fsr: false }))).toBe(false);
  });
});
