import { describe, it, expect } from "vitest";
import { gameTechnologies, matchesTechnology, matchesAntiCheat } from "@/lib/libraryFilters";

const dll = (family: string): { family: string } => ({ family });

describe("libraryFilters", () => {
  describe("gameTechnologies", () => {
    it("collects the family-groups present in a game's DLLs, deduped", () => {
      const got = gameTechnologies([dll("dlss_sr"), dll("dlss_fg"), dll("fsr_upscaler")]);
      expect([...got].sort()).toEqual(["dlss", "fsr"]);
    });
    it("is empty for a game with no scanned DLLs", () => {
      expect(gameTechnologies([]).size).toBe(0);
    });
  });

  describe("matchesTechnology", () => {
    it("'all' always matches", () => {
      expect(matchesTechnology([], "all")).toBe(true);
      expect(matchesTechnology([dll("dlss_sr")], "all")).toBe(true);
    });
    it("matches only games that contain the selected family-group", () => {
      expect(matchesTechnology([dll("dlss_sr")], "dlss")).toBe(true);
      expect(matchesTechnology([dll("dlss_sr")], "fsr")).toBe(false);
      expect(matchesTechnology([dll("fsr_upscaler")], "fsr")).toBe(true);
      expect(matchesTechnology([dll("xess_sr")], "xess")).toBe(true);
    });
    it("a game with no DLLs matches no specific group", () => {
      expect(matchesTechnology([], "dlss")).toBe(false);
    });
  });

  describe("matchesAntiCheat", () => {
    const flagged = new Set(["a"]);
    it("'all' always matches regardless of flag state", () => {
      expect(matchesAntiCheat("a", flagged, "all")).toBe(true);
      expect(matchesAntiCheat("z", flagged, "all")).toBe(true);
    });
    it("'flagged' matches only games in the anti-cheat set", () => {
      expect(matchesAntiCheat("a", flagged, "flagged")).toBe(true);
      expect(matchesAntiCheat("b", flagged, "flagged")).toBe(false);
    });
    it("'clear' matches only games not in the anti-cheat set", () => {
      expect(matchesAntiCheat("a", flagged, "clear")).toBe(false);
      expect(matchesAntiCheat("b", flagged, "clear")).toBe(true);
    });
  });
});
