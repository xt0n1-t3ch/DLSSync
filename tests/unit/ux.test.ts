import { describe, it, expect } from "vitest";
import {
  COMMANDS,
  matchCommands,
  pushRecentCommand,
  isModifierComboMatch,
  vendorForFamily,
  githubReleaseTagUrl,
  matchedIndices,
  highlightSegments,
  EXTERNAL_URLS,
  COMMAND_PALETTE_RECENT_MAX,
  COMMAND_CATEGORY_LABELS,
} from "@/lib/ux";

describe("matchCommands", () => {
  it("empty query returns every command in original order, score 0", () => {
    const out = matchCommands("", COMMANDS);
    expect(out.length).toBe(COMMANDS.length);
    expect(out.every((m) => m.score === 0)).toBe(true);
  });

  it("exact substring ranks above subsequence fuzz", () => {
    const out = matchCommands("library", COMMANDS);
    expect(out[0].command.id).toBe("nav.library");
  });

  it("matches against aliases too", () => {
    const out = matchCommands("snapshots", COMMANDS);
    expect(out[0].command.id).toBe("nav.backups");
  });

  it("fuzzy subsequence matches non-contiguous characters", () => {
    const out = matchCommands("gtl", COMMANDS);
    expect(out.some((m) => m.command.id === "nav.library")).toBe(true);
  });

  it("nonsense query returns no matches", () => {
    expect(matchCommands("zzqzzq", COMMANDS).length).toBe(0);
  });

  it("query is trimmed and lowercased", () => {
    expect(matchCommands("  CATALOG  ", COMMANDS)[0].command.id).toBe("nav.catalog");
  });
});

describe("pushRecentCommand", () => {
  it("prepends new id", () => {
    expect(pushRecentCommand([], "a")).toEqual(["a"]);
    expect(pushRecentCommand(["b", "c"], "a")).toEqual(["a", "b", "c"]);
  });

  it("dedupes by moving an existing id to the front", () => {
    expect(pushRecentCommand(["b", "a", "c"], "a")).toEqual(["a", "b", "c"]);
  });

  it("caps at COMMAND_PALETTE_RECENT_MAX", () => {
    const many = ["a", "b", "c", "d", "e", "f", "g"];
    const out = pushRecentCommand(many, "z");
    expect(out.length).toBe(COMMAND_PALETTE_RECENT_MAX);
    expect(out[0]).toBe("z");
  });
});

describe("isModifierComboMatch", () => {
  const ev = (init: Partial<KeyboardEvent>): KeyboardEvent =>
    ({ key: "k", metaKey: false, ctrlKey: false, shiftKey: false, ...init }) as KeyboardEvent;

  it("matches mod+k via ctrl or meta", () => {
    expect(isModifierComboMatch(ev({ key: "k", ctrlKey: true }), ["mod", "k"])).toBe(true);
    expect(isModifierComboMatch(ev({ key: "k", metaKey: true }), ["mod", "k"])).toBe(true);
  });

  it("rejects bare key when mod is required", () => {
    expect(isModifierComboMatch(ev({ key: "k" }), ["mod", "k"])).toBe(false);
  });

  it("rejects mod when not requested", () => {
    expect(isModifierComboMatch(ev({ key: "k", ctrlKey: true }), ["k"])).toBe(false);
  });

  it("honours shift requirement both ways", () => {
    expect(isModifierComboMatch(ev({ key: "k", ctrlKey: true, shiftKey: true }), ["mod", "shift", "k"])).toBe(true);
    expect(isModifierComboMatch(ev({ key: "k", ctrlKey: true, shiftKey: false }), ["mod", "shift", "k"])).toBe(false);
  });

  it("esc alias maps to Escape key", () => {
    expect(isModifierComboMatch(ev({ key: "Escape" }), ["esc"])).toBe(true);
  });

  it("is case-insensitive on the key", () => {
    expect(isModifierComboMatch(ev({ key: "K", ctrlKey: true }), ["mod", "k"])).toBe(true);
  });

  it("empty key list never matches", () => {
    expect(isModifierComboMatch(ev({ key: "k" }), [])).toBe(false);
  });
});

describe("vendorForFamily", () => {
  it("routes families to vendors, direct_sr to microsoft, unknown to null", () => {
    expect(vendorForFamily("dlss_sr")).toBe("nvidia");
    expect(vendorForFamily("streamline_direct_sr")).toBe("microsoft");
    expect(vendorForFamily("xess_fg")).toBe("intel");
    expect(vendorForFamily("fsr_upscaler")).toBe("amd");
    expect(vendorForFamily("direct_storage")).toBe("microsoft");
    expect(vendorForFamily("nonsense")).toBeNull();
  });
});

describe("release + external URLs", () => {
  it("githubReleaseTagUrl builds a tag URL and strips a leading v", () => {
    expect(githubReleaseTagUrl("1.6.0")).toBe("https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.0");
    expect(githubReleaseTagUrl("v1.6.0")).toBe("https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.0");
    expect(githubReleaseTagUrl("  V2.0.1 ")).toBe("https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v2.0.1");
  });

  it("exposes the GitHub releases-latest and Nexus mod links", () => {
    expect(EXTERNAL_URLS.releasesLatest).toBe("https://github.com/xt0n1-t3ch/DLSSync/releases/latest");
    expect(EXTERNAL_URLS.nexusMod).toBe("https://www.nexusmods.com/site/mods/1922");
  });
});

describe("matchedIndices + highlightSegments (fuzzy highlight)", () => {
  it("returns a contiguous span for a substring match", () => {
    expect(matchedIndices("catalog", "Go to Catalog")).toEqual([6, 7, 8, 9, 10, 11, 12]);
  });
  it("returns subsequence positions when not contiguous", () => {
    expect(matchedIndices("gtl", "Go to Library")).toEqual([0, 3, 6]);
  });
  it("returns [] when the query is not a subsequence of the text", () => {
    expect(matchedIndices("zzz", "Go to Library")).toEqual([]);
    expect(matchedIndices("", "anything")).toEqual([]);
  });
  it("splits text into hit / non-hit segments preserving original order and casing", () => {
    const segs = highlightSegments("Go to Catalog", [6, 7, 8, 9, 10, 11, 12]);
    expect(segs).toEqual([
      { text: "Go to ", hit: false },
      { text: "Catalog", hit: true },
    ]);
    expect(segs.map((s) => s.text).join("")).toBe("Go to Catalog");
  });
  it("no indices yields a single non-hit segment", () => {
    expect(highlightSegments("Plain", [])).toEqual([{ text: "Plain", hit: false }]);
  });
});

describe("command catalog integrity", () => {
  it("command ids are unique", () => {
    const ids = COMMANDS.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
  it("every command has a category with a label", () => {
    for (const c of COMMANDS) {
      expect(COMMAND_CATEGORY_LABELS[c.category]).toBeTruthy();
    }
  });
  it("every command declares a non-empty icon key", () => {
    for (const c of COMMANDS) {
      expect(typeof c.icon).toBe("string");
      expect(c.icon.length).toBeGreaterThan(0);
    }
  });
});
