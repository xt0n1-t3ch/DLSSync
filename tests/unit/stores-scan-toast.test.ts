import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import type { DetectedGame, DllRecord } from "@/lib/api";

const detectDlls = vi.fn<(installDir: string) => Promise<DllRecord[]>>();
const detectDlssEnabler = vi.fn<(installDir: string) => Promise<boolean>>(async () => false);

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    detectDlls: (dir: string) => detectDlls(dir),
    detectDlssEnabler: (dir: string) => detectDlssEnabler(dir),
  };
});

function game(id: string): DetectedGame {
  return {
    id,
    name: id,
    launcher: "steam",
    install_dir: `C:\\Games\\${id}`,
    image_url: null,
  } as DetectedGame;
}

import { games, gameDllErrors, gameDlls, toasts, rescanGame } from "@/lib/stores";

describe("rescanGame failure toast (F4)", () => {
  beforeEach(() => {
    detectDlls.mockReset();
    detectDlssEnabler.mockReset();
    detectDlssEnabler.mockResolvedValue(false);
    games.set([]);
    gameDllErrors.set({});
    gameDlls.set({});
    toasts.set([]);
  });

  it("surfaces a warning toast when the single game's rescan fails", async () => {
    games.set([game("g1")]);
    detectDlls.mockRejectedValue(new Error("locked"));

    await rescanGame("g1");

    expect(get(gameDllErrors).g1).toBeTruthy();
    const warnings = get(toasts).filter((t) => t.kind === "warning");
    expect(warnings.length).toBe(1);
  });

  it("does not toast when the rescan succeeds", async () => {
    games.set([game("g2")]);
    detectDlls.mockResolvedValue([]);

    await rescanGame("g2");

    expect(get(gameDllErrors).g2).toBeNull();
    expect(get(toasts).filter((t) => t.kind === "warning").length).toBe(0);
  });

  it("does not toast for an unknown game id (no-op)", async () => {
    detectDlls.mockResolvedValue([]);

    await rescanGame("missing");

    expect(detectDlls).not.toHaveBeenCalled();
    expect(get(toasts).length).toBe(0);
  });

  it("re-toasts on a second failing rescan — the flag is no longer module-sticky", async () => {
    games.set([game("g3")]);
    detectDlls.mockRejectedValue(new Error("locked"));

    await rescanGame("g3");
    toasts.set([]);
    await rescanGame("g3");

    expect(get(toasts).filter((t) => t.kind === "warning").length).toBe(1);
  });
});
