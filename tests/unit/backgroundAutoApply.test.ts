import { describe, it, expect, vi, beforeEach } from "vitest";
import type { AntiCheatReport, DetectedGame } from "@/lib/api";
import type { ApplyTarget } from "@/lib/applyController";
import type { OutdatedDllItem } from "@/lib/stores";

const detectAnticheat = vi.fn<(dir: string, appId: string | null, name: string) => Promise<AntiCheatReport>>();
const dispatchApply = vi.fn(async () => null);

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return { ...actual, detectAnticheat: (...a: Parameters<typeof detectAnticheat>) => detectAnticheat(...a) };
});

vi.mock("@/lib/applyController", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/applyController")>();
  return {
    ...actual,
    dispatchApply: (...a: unknown[]) => (dispatchApply as unknown as (...x: unknown[]) => Promise<null>)(...a),
  };
});

const cleanReport: AntiCheatReport = { detected: [], status: null, source_url: null };
const flaggedReport: AntiCheatReport = {
  detected: [{ anticheat: "EAC", kind: "anti_cheat", source: "dataset" }],
  status: null,
  source_url: null,
};

function game(id: string): DetectedGame {
  return {
    id,
    name: id,
    launcher: "steam",
    install_dir: `C:\\Games\\${id}`,
    app_id: null,
    image_url: null,
    size_bytes: null,
  };
}

function item(gameId: string): OutdatedDllItem {
  return {
    game: game(gameId),
    record: {
      family: "dlss_sr",
      path: `C:\\Games\\${gameId}\\nvngx_dlss.dll`,
      current_version: "1.0.0.0",
      file_description: null,
      sha256: null,
    },
    target: "2.0.0.0",
  };
}

let autoApplyExcludingAntiCheat: typeof import("@/lib/backgroundScan").autoApplyExcludingAntiCheat;

beforeEach(async () => {
  detectAnticheat.mockReset();
  dispatchApply.mockClear();
  ({ autoApplyExcludingAntiCheat } = await import("@/lib/backgroundScan"));
});

describe("autoApplyExcludingAntiCheat", () => {
  it("no-ops on an empty set", async () => {
    await autoApplyExcludingAntiCheat([]);
    expect(dispatchApply).not.toHaveBeenCalled();
  });

  it("dispatches every item when no game is anti-cheat flagged", async () => {
    detectAnticheat.mockResolvedValue(cleanReport);
    await autoApplyExcludingAntiCheat([item("a"), item("b")]);
    expect(dispatchApply).toHaveBeenCalledTimes(1);
    const targets = dispatchApply.mock.calls[0][0] as ApplyTarget[];
    expect(targets.map((t) => t.game_id).sort()).toEqual(["a", "b"]);
  });

  it("excludes anti-cheat-flagged games from the batch", async () => {
    detectAnticheat.mockImplementation(async (_dir, _appId, name) =>
      name === "blocked" ? flaggedReport : cleanReport,
    );
    await autoApplyExcludingAntiCheat([item("safe"), item("blocked")]);
    expect(dispatchApply).toHaveBeenCalledTimes(1);
    const targets = dispatchApply.mock.calls[0][0] as ApplyTarget[];
    expect(targets.map((t) => t.game_id)).toEqual(["safe"]);
  });

  it("dispatches nothing when every pending game is anti-cheat flagged", async () => {
    detectAnticheat.mockResolvedValue(flaggedReport);
    await autoApplyExcludingAntiCheat([item("a"), item("b")]);
    expect(dispatchApply).not.toHaveBeenCalled();
  });

  it("keeps a game in (relies on backend guards) when the anti-cheat probe throws", async () => {
    detectAnticheat.mockRejectedValue(new Error("probe boom"));
    await autoApplyExcludingAntiCheat([item("a")]);
    expect(dispatchApply).toHaveBeenCalledTimes(1);
    const targets = dispatchApply.mock.calls[0][0] as ApplyTarget[];
    expect(targets.map((t) => t.game_id)).toEqual(["a"]);
  });

  it("probes each distinct game once, not once per DLL", async () => {
    detectAnticheat.mockResolvedValue(cleanReport);
    const a1 = item("a");
    const a2 = item("a");
    await autoApplyExcludingAntiCheat([a1, a2]);
    expect(detectAnticheat).toHaveBeenCalledTimes(1);
  });
});
