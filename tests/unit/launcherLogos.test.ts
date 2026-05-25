import { describe, it, expect } from "vitest";
import { LAUNCHER_BRANDS, LAUNCHER_BRAND_ORDER } from "@/lib/launcherLogos";

describe("LAUNCHER_BRANDS", () => {
  it("covers the seven supported launchers", () => {
    expect(Object.keys(LAUNCHER_BRANDS).sort()).toEqual(
      ["battlenet", "ea_desktop", "epic", "gog", "steam", "ubisoft", "xbox"].sort(),
    );
  });

  it("each brand has a non-empty label, a valid hex bg, and a path", () => {
    for (const [key, brand] of Object.entries(LAUNCHER_BRANDS)) {
      expect(brand.label, `${key} label`).toBeTruthy();
      expect(brand.bg, `${key} bg`).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(brand.path.length, `${key} path`).toBeGreaterThan(20);
      expect(brand.path.startsWith("M"), `${key} path starts with moveto`).toBe(true);
    }
  });

  it("order list references only real brand keys and has no dupes", () => {
    expect(LAUNCHER_BRAND_ORDER.length).toBe(Object.keys(LAUNCHER_BRANDS).length);
    expect(new Set(LAUNCHER_BRAND_ORDER).size).toBe(LAUNCHER_BRAND_ORDER.length);
    for (const k of LAUNCHER_BRAND_ORDER) {
      expect(LAUNCHER_BRANDS[k]).toBeDefined();
    }
  });
});
