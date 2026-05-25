import { describe, it, expect } from "vitest";
import type { DllRecord } from "@/lib/api";
import { gameStatusFromRecords, buildShasByVendor, type RelationContext } from "@/lib/relation";
import type { UpdateStatus } from "@/lib/labels";

function rec(family: string, version: string, path: string, sha: string | null = null): DllRecord {
  return { family, path, current_version: version, sha256: sha } as DllRecord;
}

const shas = {
  "nvidia::dlss_sr::nvngx_dlss.dll": "aaa",
};
const ctx: RelationContext = {
  latestByKey: { dlss_sr: "2.0.0.0", fsr_upscaler: "3.1.0", xess_sr: "1.3.0" },
  shas,
  shasByVendor: buildShasByVendor(shas),
};

interface Game {
  id: string;
  name: string;
  records?: DllRecord[];
  scanError?: string | null;
}

const library: Game[] = [
  { id: "g-cp", name: "Cyberpunk 2077", records: [rec("dlss_sr", "1.0.0.0", "x/nvngx_dlss.dll")] },
  { id: "g-ac", name: "Assassins Creed", records: [rec("fsr_upscaler", "3.1.0", "x/amd.dll")] },
  { id: "g-zoo", name: "Zoo Tycoon", records: [] },
  { id: "g-broke", name: "Broken Scan", records: [rec("dlss_sr", "1.0.0.0", "x/nvngx_dlss.dll")], scanError: "registry read failed" },
  { id: "g-sha", name: "Sha Match Game", records: [rec("dlss_sr", "1.0.0.0", "x/nvngx_dlss.dll", "AAA")] },
];

describe("library status derivation (relation + labels integration)", () => {
  it("derives the correct UpdateStatus per game", () => {
    const status = (g: Game): UpdateStatus =>
      gameStatusFromRecords(g.records, ctx, [], g.scanError ?? null);
    expect(status(library[0])).toBe("outdated");
    expect(status(library[1])).toBe("up_to_date");
    expect(status(library[2])).toBe("no_dlls");
    expect(status(library[3])).toBe("scan_failed");
    expect(status(library[4])).toBe("up_to_date");
  });
});

describe("outdated-first sort policy (the ordering the Library Sort applies)", () => {
  const STATUS_SORT_RANK: Record<string, number> = {
    outdated: 0,
    scan_failed: 1,
    up_to_date: 2,
    no_dlls: 3,
    unknown: 4,
    scanning: 5,
  };

  function byOutdatedThenName(a: Game, b: Game): number {
    const sa = gameStatusFromRecords(a.records, ctx, [], a.scanError ?? null);
    const sb = gameStatusFromRecords(b.records, ctx, [], b.scanError ?? null);
    const ra = STATUS_SORT_RANK[sa] ?? 9;
    const rb = STATUS_SORT_RANK[sb] ?? 9;
    return ra - rb || a.name.localeCompare(b.name);
  }

  it("puts outdated first, then scan_failed, then up_to_date, then no_dlls; alpha within a tier", () => {
    const sorted = [...library].sort(byOutdatedThenName).map((g) => g.id);
    expect(sorted).toEqual(["g-cp", "g-broke", "g-ac", "g-sha", "g-zoo"]);
  });

  it("is a stable ordering for equal status (alphabetical by name)", () => {
    const sameTier: Game[] = [
      { id: "b", name: "Bravo", records: [rec("fsr_upscaler", "3.1.0", "x/a.dll")] },
      { id: "a", name: "Alpha", records: [rec("fsr_upscaler", "3.1.0", "x/a.dll")] },
    ];
    expect([...sameTier].sort(byOutdatedThenName).map((g) => g.name)).toEqual(["Alpha", "Bravo"]);
  });
});
