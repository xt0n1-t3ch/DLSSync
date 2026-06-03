import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  handleScanTick,
  handleApplyAll,
  type BackgroundDeps,
} from "@/lib/backgroundScan";
import type { OutdatedDllItem } from "@/lib/stores";
import type { DetectedGame } from "@/lib/api";
import { BACKGROUND_SCAN_TICK_EVENT, BACKGROUND_APPLY_ALL_EVENT } from "@/lib/api";

function game(id: string, over: Partial<DetectedGame> = {}): DetectedGame {
  return {
    id,
    name: id,
    launcher: "steam",
    install_dir: `C:\\Games\\${id}`,
    app_id: null,
    image_url: null,
    size_bytes: null,
    ...over,
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

const CONFIG_DEFAULT = {
  enabled: true,
  interval_hours: 24,
  close_to_tray: false,
  run_at_startup: false,
  notify_os_toast: true,
  auto_apply: false,
};

function makeDeps(over: Partial<BackgroundDeps> = {}, cfg: Partial<typeof CONFIG_DEFAULT> = {}): BackgroundDeps {
  return {
    isScanning: () => false,
    isApplyInflight: () => false,
    scanGames: vi.fn(async () => undefined),
    loadCatalog: vi.fn(async () => undefined),
    emitDigest: vi.fn(),
    pendingDigest: () => ({ total: 0, games: 0 }),
    outdatedItems: () => [],
    config: () => ({ ...CONFIG_DEFAULT, ...cfg }),
    setTrayPending: vi.fn(async () => undefined),
    notifyToast: vi.fn(async () => true),
    autoApply: vi.fn(async () => undefined),
    triggerApplyAll: vi.fn(),
    ...over,
  };
}

describe("handleScanTick — guards", () => {
  it("no-ops entirely when the daemon is disabled", async () => {
    const scanGames = vi.fn(async () => undefined);
    const deps = makeDeps({ scanGames }, { enabled: false });
    await handleScanTick(deps);
    expect(scanGames).not.toHaveBeenCalled();
    expect(deps.setTrayPending).not.toHaveBeenCalled();
  });

  it("skips when a scan is already in progress", async () => {
    const scanGames = vi.fn(async () => undefined);
    const deps = makeDeps({ isScanning: () => true, scanGames });
    await handleScanTick(deps);
    expect(scanGames).not.toHaveBeenCalled();
  });

  it("skips when an apply is inflight", async () => {
    const scanGames = vi.fn(async () => undefined);
    const deps = makeDeps({ isApplyInflight: () => true, scanGames });
    await handleScanTick(deps);
    expect(scanGames).not.toHaveBeenCalled();
  });
});

describe("handleScanTick — scan + tray", () => {
  it("runs scan then catalog refresh and reports the games count to the tray", async () => {
    const order: string[] = [];
    const scanGames = vi.fn(async () => { order.push("scan"); });
    const loadCatalog = vi.fn(async () => { order.push("catalog"); });
    const setTrayPending = vi.fn(async () => undefined);
    const deps = makeDeps({
      scanGames,
      loadCatalog,
      setTrayPending,
      pendingDigest: () => ({ total: 5, games: 2 }),
    });
    await handleScanTick(deps);
    expect(order).toEqual(["scan", "catalog"]);
    expect(setTrayPending).toHaveBeenCalledWith(2);
  });

  it("reports zero to the tray and shows no toast when nothing is pending", async () => {
    const notifyToast = vi.fn(async () => true);
    const setTrayPending = vi.fn(async () => undefined);
    const deps = makeDeps({
      notifyToast,
      setTrayPending,
      pendingDigest: () => ({ total: 0, games: 0 }),
    });
    await handleScanTick(deps);
    expect(setTrayPending).toHaveBeenCalledWith(0);
    expect(notifyToast).not.toHaveBeenCalled();
  });
});

describe("handleScanTick — toast", () => {
  it("fires a native toast only when there are pending updates", async () => {
    const notifyToast = vi.fn(async () => true);
    const deps = makeDeps({ notifyToast, pendingDigest: () => ({ total: 3, games: 1 }) });
    await handleScanTick(deps);
    expect(notifyToast).toHaveBeenCalledTimes(1);
  });

  it("falls back to the in-app digest when the OS toast is denied", async () => {
    const emitDigest = vi.fn();
    const notifyToast = vi.fn(async () => false);
    const deps = makeDeps({ emitDigest, notifyToast, pendingDigest: () => ({ total: 3, games: 1 }) });
    await handleScanTick(deps);
    expect(notifyToast).toHaveBeenCalledTimes(1);
    expect(emitDigest).toHaveBeenCalledTimes(1);
  });

  it("uses the in-app digest (no native toast) when notify_os_toast is off", async () => {
    const emitDigest = vi.fn();
    const notifyToast = vi.fn(async () => true);
    const deps = makeDeps(
      { emitDigest, notifyToast, pendingDigest: () => ({ total: 3, games: 1 }) },
      { notify_os_toast: false },
    );
    await handleScanTick(deps);
    expect(notifyToast).not.toHaveBeenCalled();
    expect(emitDigest).toHaveBeenCalledTimes(1);
  });
});

describe("handleScanTick — auto-apply", () => {
  it("does not auto-apply when auto_apply is off", async () => {
    const autoApply = vi.fn(async () => undefined);
    const deps = makeDeps(
      { autoApply, pendingDigest: () => ({ total: 2, games: 1 }) },
      { auto_apply: false },
    );
    await handleScanTick(deps);
    expect(autoApply).not.toHaveBeenCalled();
  });

  it("auto-applies the outdated set when auto_apply is on and updates exist", async () => {
    const outItems = [item("a"), item("b")];
    const autoApply = vi.fn(async () => undefined);
    const deps = makeDeps(
      {
        autoApply,
        outdatedItems: () => outItems,
        pendingDigest: () => ({ total: 2, games: 2 }),
      },
      { auto_apply: true },
    );
    await handleScanTick(deps);
    expect(autoApply).toHaveBeenCalledTimes(1);
    expect(autoApply).toHaveBeenCalledWith(outItems);
  });

  it("never auto-applies when nothing is pending, even with auto_apply on", async () => {
    const autoApply = vi.fn(async () => undefined);
    const deps = makeDeps(
      { autoApply, pendingDigest: () => ({ total: 0, games: 0 }) },
      { auto_apply: true },
    );
    await handleScanTick(deps);
    expect(autoApply).not.toHaveBeenCalled();
  });
});

describe("handleApplyAll", () => {
  it("delegates to the shared Apply-All trigger", () => {
    const triggerApplyAll = vi.fn();
    handleApplyAll(makeDeps({ triggerApplyAll }));
    expect(triggerApplyAll).toHaveBeenCalledTimes(1);
  });
});

describe("event-name contract", () => {
  it("matches the backend tick + apply-all literals exactly", () => {
    expect(BACKGROUND_SCAN_TICK_EVENT).toBe("background:scan-tick");
    expect(BACKGROUND_APPLY_ALL_EVENT).toBe("background:apply-all");
  });
});
