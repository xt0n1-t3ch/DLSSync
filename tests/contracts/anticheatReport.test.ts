import { describe, it, expect } from "vitest";
import schema from "../../contracts/anticheat-report.schema.json";
import { assertConforms } from "./_schema";

const detectedReport = {
  detected: [
    { anticheat: "Easy Anti-Cheat", kind: "anti_cheat", source: "binary" },
    { anticheat: "Denuvo Anti-Tamper", kind: "anti_tamper", source: "pe" },
    { anticheat: "BattlEye", kind: "anti_cheat", source: "dataset" },
  ],
  status: "Broken",
  source_url: "https://www.pcgamingwiki.com/api/appid.php?appid=3159330",
};

const cleanReport = {
  detected: [],
  status: null,
  source_url: null,
};

describe("anticheat-report contract", () => {
  it("documents the agreed field set as required", () => {
    expect(schema.required).toEqual(["detected", "status", "source_url"]);
  });

  it.each([
    ["a detected report", detectedReport],
    ["a clean report", cleanReport],
  ])("validates %s against the schema", (_label, fixture) => {
    assertConforms(fixture, schema as never);
  });

  it("rejects an unknown detection source", () => {
    expect(() =>
      assertConforms(
        {
          detected: [{ anticheat: "X", kind: "anti_cheat", source: "guess" }],
          status: null,
          source_url: null,
        },
        schema as never,
      ),
    ).toThrow();
  });

  it("rejects an unknown protection kind", () => {
    expect(() =>
      assertConforms(
        {
          detected: [{ anticheat: "X", kind: "mystery", source: "pe" }],
          status: null,
          source_url: null,
        },
        schema as never,
      ),
    ).toThrow();
  });
});
