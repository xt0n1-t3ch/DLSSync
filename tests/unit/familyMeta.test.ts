import { describe, it, expect } from "vitest";
import type { DllFamily } from "@/lib/api";
import { FAMILY_META, familyMeta, type VendorKey } from "@/lib/familyMeta";
import { vendorForFamily, VENDOR_TOKEN_BY_FAMILY } from "@/lib/ux";
import { FAMILY_TO_VENDOR } from "@/lib/labels";

/** Mirror of the `DllFamily` union in api.ts. Kept as an explicit list so a new
 *  family added to the union fails this test until it is added here AND to
 *  FAMILY_META — making the table the single place that must stay exhaustive. */
const ALL_FAMILIES: readonly DllFamily[] = [
  "dlss_sr",
  "dlss_fg",
  "dlss_rr",
  "sl_dlss_sr",
  "sl_dlss_fg",
  "sl_dlss_rr",
  "streamline",
  "streamline_common",
  "streamline_pcl",
  "streamline_nis",
  "streamline_direct_sr",
  "reflex",
  "xess_sr",
  "xess_sr_dx11",
  "xess_fg",
  "xell",
  "fsr_upscaler",
  "fsr_upscaler_vk",
  "fsr_fg",
  "fsr_loader",
  "fsr_denoiser",
  "direct_storage",
  "direct_storage_core",
];

const VALID_VENDORS: readonly VendorKey[] = ["nvidia", "amd", "intel", "microsoft"];

describe("FAMILY_META exhaustiveness", () => {
  it("covers exactly the DllFamily union with no extras or gaps", () => {
    const metaKeys = Object.keys(FAMILY_META).sort();
    expect(metaKeys).toEqual([...ALL_FAMILIES].sort());
  });

  it("assigns every family exactly one valid vendor", () => {
    for (const family of ALL_FAMILIES) {
      const m = FAMILY_META[family];
      expect(VALID_VENDORS).toContain(m.vendor);
    }
  });

  it("derives the token as var(--vendor-<vendor>) for every family", () => {
    for (const family of ALL_FAMILIES) {
      const m = FAMILY_META[family];
      expect(m.token).toBe(`var(--vendor-${m.vendor})`);
    }
  });
});

describe("vendor resolution is consistent across all three tables", () => {
  it("vendorForFamily, FAMILY_TO_VENDOR and FAMILY_META agree for every family", () => {
    for (const family of ALL_FAMILIES) {
      const canonical = FAMILY_META[family].vendor;
      expect(vendorForFamily(family)).toBe(canonical);
      expect(FAMILY_TO_VENDOR[family]).toBe(canonical);
    }
  });

  it("VENDOR_TOKEN_BY_FAMILY token matches the canonical vendor token", () => {
    for (const family of ALL_FAMILIES) {
      expect(VENDOR_TOKEN_BY_FAMILY[family]).toBe(FAMILY_META[family].token);
    }
  });
});

describe("the formerly-disagreeing families resolve one way now", () => {
  it("streamline_direct_sr is microsoft everywhere (DirectSR wrapped in Streamline)", () => {
    expect(vendorForFamily("streamline_direct_sr")).toBe("microsoft");
    expect(FAMILY_TO_VENDOR["streamline_direct_sr"]).toBe("microsoft");
    expect(FAMILY_META.streamline_direct_sr.vendor).toBe("microsoft");
  });

  it("direct_storage_core is microsoft everywhere", () => {
    expect(vendorForFamily("direct_storage_core")).toBe("microsoft");
    expect(FAMILY_TO_VENDOR["direct_storage_core"]).toBe("microsoft");
    expect(FAMILY_META.direct_storage_core.vendor).toBe("microsoft");
  });
});

describe("familyMeta lookup edge cases", () => {
  it("returns the canonical meta for a known family", () => {
    expect(familyMeta("dlss_sr")).toEqual(FAMILY_META.dlss_sr);
  });

  it("returns null for an unknown string", () => {
    expect(familyMeta("not_a_family")).toBeNull();
  });

  it("returns null for an empty string", () => {
    expect(familyMeta("")).toBeNull();
  });

  it("vendorForFamily returns null for unknown / empty input", () => {
    expect(vendorForFamily("nonsense")).toBeNull();
    expect(vendorForFamily("")).toBeNull();
  });
});
