import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import {
  BACKGROUND_SCAN_TICK_EVENT,
  BACKGROUND_APPLY_ALL_EVENT,
  detectAnticheat,
  traySetPending,
  type DetectedGame,
} from "./api";
import {
  scanGames as scanGamesStore,
  loadCatalog as loadCatalogStore,
  scanInProgress,
  inflightCount,
  pendingDllUpdateDigest,
  outdatedDllItems,
  emitDllUpdatesDigest,
  backgroundConfig,
  triggerApplyAllOutdated,
  applyModalOpen,
  showToast,
  type OutdatedDllItem,
} from "./stores";
import {
  dispatchApply,
  buildTargetFromRecord,
  isApplyInflight as trackersInflight,
  type ApplyTarget,
} from "./applyController";
import { hasAntiCheat } from "./anticheat";
import { translate, locale } from "./i18n/index";

/** Injection seam so the tick/apply-all handlers can be unit-tested without the
 *  Tauri runtime. The defaults wire the real store/API functions. */
export interface BackgroundDeps {
  isScanning(): boolean;
  isApplyInflight(): boolean;
  scanGames(): Promise<void>;
  loadCatalog(): Promise<void>;
  emitDigest(): void;
  pendingDigest(): { total: number; games: number };
  outdatedItems(): OutdatedDllItem[];
  config(): ReturnType<typeof backgroundConfig>;
  setTrayPending(count: number): Promise<void>;
  notifyToast(title: string, body: string): Promise<boolean>;
  autoApply(items: OutdatedDllItem[]): Promise<void>;
  triggerApplyAll(): void;
}

let toastPermissionResolved = false;
let toastPermissionGranted = false;

/** Request OS-notification permission once per session. Returns whether toasts
 *  may be shown. A denial (or a missing plugin) is cached so we fall back to the
 *  in-app digest without re-prompting. */
async function ensureToastPermission(): Promise<boolean> {
  if (toastPermissionResolved) return toastPermissionGranted;
  toastPermissionResolved = true;
  try {
    const { isPermissionGranted, requestPermission } = await import(
      "@tauri-apps/plugin-notification"
    );
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    toastPermissionGranted = granted;
  } catch (err: unknown) {
    console.warn("[dlssync] notification permission unavailable:", err);
    toastPermissionGranted = false;
  }
  return toastPermissionGranted;
}

/** Fire a native Windows toast. Returns false (without throwing) when permission
 *  was denied or the plugin is unavailable, so the caller can fall back. */
async function fireNativeToast(title: string, body: string): Promise<boolean> {
  if (!(await ensureToastPermission())) return false;
  try {
    const { sendNotification } = await import("@tauri-apps/plugin-notification");
    sendNotification({ title, body });
    return true;
  } catch (err: unknown) {
    console.warn("[dlssync] native toast failed:", err);
    return false;
  }
}

/** Default deps: drive the live stores/APIs. */
function liveDeps(): BackgroundDeps {
  return {
    isScanning: () => get(scanInProgress),
    isApplyInflight: () => get(inflightCount) > 0 || trackersInflight(),
    scanGames: () => scanGamesStore({ silent: true }),
    loadCatalog: () => loadCatalogStore({ silent: true }),
    emitDigest: () => emitDllUpdatesDigest(),
    pendingDigest: () => pendingDllUpdateDigest(),
    outdatedItems: () => outdatedDllItems(),
    config: () => backgroundConfig(),
    setTrayPending: (count) => traySetPending(count),
    notifyToast: (title, body) => fireNativeToast(title, body),
    autoApply: (items) => autoApplyExcludingAntiCheat(items),
    triggerApplyAll: () => triggerApplyAllOutdated(),
  };
}

/** Auto-apply every outdated DLL EXCEPT those in anti-cheat-flagged games. The
 *  per-game anti-cheat probe reuses the exact API the game drawer uses; a probe
 *  failure is treated as "not flagged" (the backend apply guards remain the hard
 *  safety net). Backups/pins/Enabler/Streamline coherence are enforced by the
 *  apply path. */
export async function autoApplyExcludingAntiCheat(items: OutdatedDllItem[]): Promise<void> {
  if (items.length === 0) return;
  const byGame = new Map<string, DetectedGame>();
  for (const it of items) byGame.set(it.game.id, it.game);
  const blockedGameIds = new Set<string>();
  for (const game of byGame.values()) {
    try {
      const report = await detectAnticheat(game.install_dir, game.app_id, game.name);
      if (hasAntiCheat(report)) blockedGameIds.add(game.id);
    } catch {
    }
  }
  const allowed = items.filter((it) => !blockedGameIds.has(it.game.id));
  if (allowed.length === 0) {
    if (blockedGameIds.size > 0) {
      showToast("info", translate(get(locale), "view.library.toast.autoApplyAllSkipped"));
    }
    return;
  }
  if (blockedGameIds.size > 0) {
    showToast(
      "info",
      translate(get(locale), "view.library.toast.autoApplySkippedAnticheat", {
        count: blockedGameIds.size,
      }),
    );
  }
  const targets: ApplyTarget[] = allowed.map((it) =>
    buildTargetFromRecord(it.game, it.record, it.target),
  );
  await dispatchApply(targets, { showModal: () => applyModalOpen.set(true) });
}

/** Handle one background scan tick. Skips entirely when a scan is already running
 *  or an apply is inflight (no concurrent scan/apply). Otherwise runs the proven
 *  scan + catalog refresh flow (which emits the in-app digest), updates the tray
 *  count, fires an OS toast when there are pending updates (falling back to the
 *  in-app digest if permission is denied), and optionally auto-applies. */
let tickRunning = false;

export async function handleScanTick(deps: BackgroundDeps = liveDeps()): Promise<void> {
  if (tickRunning) return;
  const cfg = deps.config();
  if (!cfg.enabled) return;
  if (deps.isScanning() || deps.isApplyInflight()) return;

  tickRunning = true;
  try {
    await deps.scanGames();
    await deps.loadCatalog();

    const { total, games } = deps.pendingDigest();
    await deps.setTrayPending(games).catch((err) => {
      console.warn("[dlssync] tray_set_pending failed:", err);
    });

    if (total <= 0) return;

    const loc = get(locale);
    const gamesPhrase = translate(loc, "notif.dll.digestGames", { count: games });
    const title = translate(loc, "notif.background.toastTitle", { count: total });
    const body = translate(loc, "notif.background.toastBody", { games: gamesPhrase });

    if (cfg.notify_os_toast) {
      const shown = await deps.notifyToast(title, body);
      if (!shown) deps.emitDigest();
    } else {
      deps.emitDigest();
    }

    if (cfg.auto_apply) {
      await deps.autoApply(deps.outdatedItems());
    }
  } finally {
    tickRunning = false;
  }
}

/** Handle the tray "Apply all updates" action — runs the same Apply-All flow the
 *  Library header button uses (guard-gated by the apply path). */
export function handleApplyAll(deps: BackgroundDeps = liveDeps()): void {
  deps.triggerApplyAll();
}

let installed = false;
let unlisteners: UnlistenFn[] = [];

/** Install the daemon event listeners. Wired alongside the apply listeners in
 *  App.svelte. Idempotent. */
export async function installBackgroundScanListeners(): Promise<void> {
  if (installed) return;
  installed = true;
  const u1 = await listen<void>(BACKGROUND_SCAN_TICK_EVENT, () => {
    void handleScanTick();
  });
  const u2 = await listen<void>(BACKGROUND_APPLY_ALL_EVENT, () => {
    handleApplyAll();
  });
  unlisteners = [u1, u2];
}

export async function uninstallBackgroundScanListeners(): Promise<void> {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
    }
  }
  unlisteners = [];
  installed = false;
}
