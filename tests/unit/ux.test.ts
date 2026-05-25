import { describe, it, expect } from "vitest";
import {
  COMMANDS,
  matchCommands,
  pushRecentCommand,
  isModifierComboMatch,
  vendorForFamily,
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
});
