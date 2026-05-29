import { describe, it, expect } from "vitest";
import {
  BRANDS,
  brandFor,
  brandLabel,
  brandfetchLogoUrl,
  resolveBrandDomain,
  resolveBrandKey,
  type BrandKey,
} from "@/lib/brands";

describe("BRANDS", () => {
  it("bundles real official marks only for vendors with a clean open-source logo", () => {
    const keys = Object.keys(BRANDS).sort();
    expect(keys).toEqual(
      [
        "amd",
        "asus",
        "broadcom",
        "dell",
        "intel",
        "mediatek",
        "microsoft",
        "msi",
        "nvidia",
        "qualcomm",
        "razer",
      ].sort(),
    );
  });

  it("each brand has a non-empty label, a real svg path, a viewBox, and a token accent", () => {
    for (const [key, brand] of Object.entries(BRANDS)) {
      expect(brand.label, `${key} label`).toBeTruthy();
      expect(brand.path.length, `${key} path`).toBeGreaterThan(20);
      expect(brand.path.trimStart().startsWith("M"), `${key} path starts with moveto`).toBe(true);
      expect(brand.viewBox, `${key} viewBox`).toMatch(/^\d+ \d+ \d+ \d+$/);
      expect(brand.accentVar.startsWith("--"), `${key} accentVar is a css var name`).toBe(true);
    }
  });

  it("maps the core vendors onto their --vendor-* tokens", () => {
    expect(BRANDS.nvidia.accentVar).toBe("--vendor-nvidia");
    expect(BRANDS.amd.accentVar).toBe("--vendor-amd");
    expect(BRANDS.intel.accentVar).toBe("--vendor-intel");
    expect(BRANDS.microsoft.accentVar).toBe("--vendor-microsoft");
  });
});

describe("resolveBrandKey", () => {
  const cases: ReadonlyArray<readonly [string, BrandKey]> = [
    ["nvidia", "nvidia"],
    ["NVIDIA", "nvidia"],
    ["NVIDIA Corporation", "nvidia"],
    ["GeForce RTX 4070", "nvidia"],
    ["amd", "amd"],
    ["Advanced Micro Devices, Inc.", "amd"],
    ["Advanced Micro Devices, Inc. - Display", "amd"],
    ["ATI Technologies Inc.", "amd"],
    ["AMD Radeon(TM) Graphics", "amd"],
    ["Intel", "intel"],
    ["Intel Corporation", "intel"],
    ["Microsoft", "microsoft"],
    ["Microsoft Corporation", "microsoft"],
    ["Dell Inc.", "dell"],
    ["Alienware", "dell"],
    ["Micro-Star International", "msi"],
    ["MSI", "msi"],
    ["ASUSTeK Computer Inc.", "asus"],
    ["ROG", "asus"],
    ["Qualcomm Atheros Communications", "qualcomm"],
    ["Snapdragon", "qualcomm"],
    ["Razer Inc.", "razer"],
    ["Broadcom Inc.", "broadcom"],
    ["MediaTek Inc.", "mediatek"],
  ];

  for (const [raw, expected] of cases) {
    it(`maps "${raw}" -> ${expected}`, () => {
      expect(resolveBrandKey(raw)).toBe(expected);
    });
  }

  it("returns null for vendors served dynamically rather than bundled", () => {
    expect(resolveBrandKey("Realtek Semiconductor Corp.")).toBeNull();
    expect(resolveBrandKey("Nahimic")).toBeNull();
    expect(resolveBrandKey("GIGABYTE")).toBeNull();
    expect(resolveBrandKey("Logitech")).toBeNull();
  });

  it("returns null for an unknown provider", () => {
    expect(resolveBrandKey("Acme Widgets LLC")).toBeNull();
    expect(resolveBrandKey("Standard system devices")).toBeNull();
    expect(resolveBrandKey("other")).toBeNull();
  });

  it("returns null for empty, null, or undefined input", () => {
    expect(resolveBrandKey("")).toBeNull();
    expect(resolveBrandKey(null)).toBeNull();
    expect(resolveBrandKey(undefined)).toBeNull();
  });
});

describe("resolveBrandDomain", () => {
  const cases: ReadonlyArray<readonly [string, string]> = [
    ["Realtek Semiconductor Corp.", "realtek.com"],
    ["Nahimic", "nahimic.com"],
    ["A-Volute", "nahimic.com"],
    ["Synaptics", "synaptics.com"],
    ["GIGABYTE", "gigabyte.com"],
    ["AORUS", "gigabyte.com"],
    ["Logitech", "logitech.com"],
    ["Advanced Micro Devices, Inc.", "amd.com"],
    ["NVIDIA Corporation", "nvidia.com"],
    ["Microsoft Corporation", "microsoft.com"],
  ];

  for (const [raw, expected] of cases) {
    it(`maps "${raw}" -> ${expected}`, () => {
      expect(resolveBrandDomain(raw)).toBe(expected);
    });
  }

  it("returns null for an unmappable provider and for empty input", () => {
    expect(resolveBrandDomain("Acme Widgets LLC")).toBeNull();
    expect(resolveBrandDomain(null)).toBeNull();
    expect(resolveBrandDomain("")).toBeNull();
  });
});

describe("brandfetchLogoUrl", () => {
  it("builds a brandfetch CDN url when a domain and client id are present", () => {
    const url = brandfetchLogoUrl("realtek.com", { clientId: "abc123", size: 32 });
    expect(url).toBe("https://cdn.brandfetch.io/realtek.com/w/32/h/32/icon?c=abc123");
  });

  it("returns null without a client id, and null without a domain", () => {
    expect(brandfetchLogoUrl("realtek.com", { clientId: "" })).toBeNull();
    expect(brandfetchLogoUrl(null, { clientId: "abc123" })).toBeNull();
  });
});

describe("brandLabel", () => {
  it("returns the clean brand label for a messy known provider", () => {
    expect(brandLabel("Advanced Micro Devices, Inc.")).toBe("AMD");
    expect(brandLabel("MediaTek Inc.")).toBe("MediaTek");
  });

  it("echoes the raw string for an unknown provider", () => {
    expect(brandLabel("Acme Widgets LLC")).toBe("Acme Widgets LLC");
  });
});

describe("brandFor", () => {
  it("returns the brand record for a known provider", () => {
    expect(brandFor("nvidia")?.label).toBe("NVIDIA");
  });

  it("returns null for an unknown provider", () => {
    expect(brandFor("Acme Widgets LLC")).toBeNull();
    expect(brandFor(null)).toBeNull();
  });
});
