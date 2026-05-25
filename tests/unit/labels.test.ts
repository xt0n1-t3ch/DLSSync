import { describe, it, expect } from "vitest";
import type { DllRecord } from "@/lib/api";
import {
  vendorLabel,
  familyLabel,
  familyShort,
  launcherLabel,
  familyVendor,
  familyCatalogKey,
  familyGroup,
  featureFromFamily,
  featureTitle,
  featureShort,
  featureIconId,
  filenameFromPath,
  recordFeature,
  viewTitle,
  launcherAccent,
  FAMILY_LABELS,
  FAMILY_TO_VENDOR,
  FAMILY_TO_CATALOG_KEY,
  GROUP_LABELS,
  STATUS_LABELS,
  LAUNCHER_LABELS,
} from "@/lib/labels";

describe("filenameFromPath", () => {
  it("extracts the basename from windows and posix paths", () => {
    expect(filenameFromPath("C:\\Games\\x\\nvngx_dlss.dll")).toBe("nvngx_dlss.dll");
    expect(filenameFromPath("/opt/games/x/libxess.dll")).toBe("libxess.dll");
  });
  it("a bare filename is returned unchanged", () => {
    expect(filenameFromPath("sl.dlss_g.dll")).toBe("sl.dlss_g.dll");
  });
});

describe("family mappings", () => {
  it("vendor resolves for each known family, defaults to nvidia", () => {
    expect(familyVendor("fsr_upscaler")).toBe("amd");
    expect(familyVendor("xess_sr")).toBe("intel");
    expect(familyVendor("direct_storage")).toBe("microsoft");
    expect(familyVendor("totally_unknown" as never)).toBe("nvidia");
  });

  it("catalog key collapses streamline + fsr variants", () => {
    expect(familyCatalogKey("streamline_common")).toBe("streamline");
    expect(familyCatalogKey("fsr_loader")).toBe("fsr_upscaler");
    expect(familyCatalogKey("xess_sr_dx11")).toBe("xess_sr");
  });

  it("family group buckets, unknown -> advanced", () => {
    expect(familyGroup("dlss_fg")).toBe("dlss");
    expect(familyGroup("fsr_fg")).toBe("fsr");
    expect(familyGroup("xell")).toBe("xess");
    expect(familyGroup("reflex")).toBe("advanced");
    expect(familyGroup("mystery")).toBe("advanced");
  });

  it("label/short fall back to the raw key when missing", () => {
    expect(familyLabel("dlss_sr")).toBe("DLSS Super Resolution");
    expect(familyLabel("unknown_x")).toBe("unknown_x");
    expect(familyShort("fsr_upscaler")).toBe("FSR");
    expect(familyShort("unknown_x")).toBe("unknown_x");
  });
});

describe("feature mapping", () => {
  it("featureFromFamily maps known families and defaults to advanced", () => {
    expect(featureFromFamily("dlss_rr")).toBe("dlss_rr");
    expect(featureFromFamily("xell")).toBe("xess_sr");
    expect(featureFromFamily("reflex")).toBe("advanced");
    expect(featureFromFamily("direct_storage")).toBe("advanced");
  });

  it("featureTitle/short/icon handle the advanced slot", () => {
    expect(featureTitle("advanced")).toBe(GROUP_LABELS.advanced);
    expect(featureShort("advanced")).toBe("Other");
    expect(featureIconId("advanced")).toBe("advanced");
    expect(featureTitle("dlss_sr")).toBe("DLSS Super Resolution");
  });

  it("recordFeature disambiguates streamline by filename suffix", () => {
    const mk = (path: string): DllRecord => ({ family: "streamline", path } as DllRecord);
    expect(recordFeature(mk("C:\\g\\sl.dlss_g.dll"))).toBe("dlss_fg");
    expect(recordFeature(mk("C:\\g\\sl.dlss_d.dll"))).toBe("dlss_rr");
    expect(recordFeature(mk("C:\\g\\sl.dlss.dll"))).toBe("dlss_sr");
    expect(recordFeature(mk("C:\\g\\sl.common.dll"))).toBe("advanced");
  });
});

describe("launcher + view labels", () => {
  it("each launcher kind has a label and an accent", () => {
    for (const k of Object.keys(LAUNCHER_LABELS)) {
      expect(launcherLabel(k as never)).toBeTruthy();
      expect(launcherAccent(k)).toMatch(/^#?[0-9a-fA-F]{3,8}$/);
    }
  });
  it("viewTitle resolves known views, empty for unknown", () => {
    expect(viewTitle("library")).toBe("Library");
    expect(viewTitle("nope")).toBe("");
  });
  it("vendorLabel falls back to the raw key", () => {
    expect(vendorLabel("nvidia")).toBe("NVIDIA");
    expect(vendorLabel("weirdco")).toBe("weirdco");
  });
});

describe("map completeness invariants", () => {
  it("every FAMILY_LABELS key also has a vendor and catalog key", () => {
    for (const fam of Object.keys(FAMILY_LABELS)) {
      expect(FAMILY_TO_VENDOR[fam], `vendor for ${fam}`).toBeTruthy();
      expect(FAMILY_TO_CATALOG_KEY[fam], `catalog key for ${fam}`).toBeTruthy();
    }
  });
  it("every UpdateStatus has a label", () => {
    for (const s of ["outdated", "up_to_date", "no_dlls", "unknown", "scanning", "scan_failed"] as const) {
      expect(STATUS_LABELS[s]).toBeTruthy();
    }
  });
});
