import { writable, derived, get, type Writable, type Readable } from "svelte/store";
import { translate, locale } from "./i18n/index";
import {
  scanLibraries,
  refreshCatalog,
  catalogSummary,
  catalogLatestShas as fetchCatalogLatestShas,
  listBackups,
  getSettings,
  saveSettings,
  detectDlls,
  detectDlssEnabler,
  enrichGameArt,
  fetchSteamArt,
  checkDriverUpdates,
  listDriverHistory,
  installDriver,
  scanSystemDrivers,
  installSystemDriver,
  driverInstallContext,
  type DetectedGame,
  type BackupEntry,
  type AppSettings,
  type DllRecord,
  type GameArt,
  type DriverStatusReport,
  type DriverReleaseDto,
  type DriverInstallProgress,
  type InstallStage,
  type GpuVendor,
  type SystemDeviceGroup,
  type SystemDriverUpdate,
  type SystemDriverProgress,
} from "./api";
import { sortDriverReports, hasDriverUpdate, driverPageUrl } from "./drivers";
import { vendorLabel, familyLabel, familyShort, type UpdateStatus } from "./labels";
import { buildShasByVendor, gameStatusFromRecords, type CatalogShasByVendor, type RelationContext } from "./relation";
import {
  notifications,
  pushNotification,
  makeNotificationEntry,
  type NotificationKind,
} from "./notifications";

const CATALOG_UPDATE_NOTIFICATION_CAP = 5;

export function diffCatalogLatest(
  before: Record<string, string>,
  after: Record<string, string>,
): Array<{ family: string; oldVersion: string; newVersion: string }> {
  const out: Array<{ family: string; oldVersion: string; newVersion: string }> = [];
  for (const [family, newVersion] of Object.entries(after)) {
    const oldVersion = before[family];
    if (oldVersion && oldVersion !== newVersion) {
      out.push({ family, oldVersion, newVersion });
    }
  }
  return out;
}

function alreadyEmitted(kind: NotificationKind, signature: string): boolean {
  return get(notifications).some(
    (n) => n.kind === kind && (n.title.includes(signature) || (n.body ?? "").includes(signature)),
  );
}

function emitFailureNotification(kind: NotificationKind, title: string, body: string): void {
  if (alreadyEmitted(kind, body)) return;
  const entry = makeNotificationEntry(kind, title, body);
  pushNotification(entry).catch((err) => console.warn(`[dlssync] push ${kind} notification failed:`, err));
}

function emitCatalogUpdateNotifications(
  diffs: Array<{ family: string; oldVersion: string; newVersion: string }>,
): void {
  const fresh = diffs.filter((d) => !alreadyEmitted("catalog_update_available", `${d.family} ${d.newVersion}`));
  if (fresh.length === 0) return;
  const head = fresh.slice(0, CATALOG_UPDATE_NOTIFICATION_CAP);
  const overflow = fresh.length - head.length;
  const loc = get(locale);
  for (const d of head) {
    const label = familyShort(d.family);
    const entry = makeNotificationEntry(
      "catalog_update_available",
      translate(loc, "notif.catalog.available", { label, version: d.newVersion }),
      translate(loc, "notif.catalog.wasVersion", { version: d.oldVersion }),
    );
    pushNotification(entry).catch((err) =>
      console.warn("[dlssync] push catalog-update notification failed:", err),
    );
  }
  if (overflow > 0) {
    const entry = makeNotificationEntry(
      "catalog_update_available",
      translate(loc, "notif.catalog.moreUpdates", { count: overflow }),
      translate(loc, "notif.catalog.familiesChanged", { count: head.length + overflow }),
    );
    pushNotification(entry).catch((err) =>
      console.warn("[dlssync] push catalog-update summary failed:", err),
    );
  }
}

const DRIVER_UPDATE_NOTIFICATION_CAP = 4;

function emitDriverUpdateNotifications(reports: DriverStatusReport[]): void {
  const fresh = reports
    .filter(hasDriverUpdate)
    .filter(
      (r) => !alreadyEmitted("driver_update_available", `${r.device.model} ${r.latest?.version.display ?? ""}`),
    );
  const loc = get(locale);
  for (const report of fresh.slice(0, DRIVER_UPDATE_NOTIFICATION_CAP)) {
    const version = report.latest?.version.display ?? translate(loc, "notif.driver.latestFallback");
    const entry = makeNotificationEntry(
      "driver_update_available",
      translate(loc, "notif.driver.title", { version, model: report.device.model }),
      translate(loc, "notif.driver.body", { vendor: vendorLabel(report.device.vendor) }),
      { link: driverPageUrl(report) },
    );
    pushNotification(entry).catch((err) =>
      console.warn("[dlssync] push driver-update notification failed:", err),
    );
  }
}

function emitSystemDriverUpdateNotification(groups: SystemDeviceGroup[]): void {
  const count = groups.reduce((total, group) => total + group.updates.length, 0);
  if (count === 0) return;
  const loc = get(locale);
  const title = translate(loc, "notif.systemDriver.available", { count });
  if (alreadyEmitted("system_driver_update_available", title)) return;
  const categories = groups.filter((group) => group.updates.length > 0).length;
  const entry = makeNotificationEntry(
    "system_driver_update_available",
    title,
    translate(loc, "notif.systemDriver.categories", { count: categories }),
  );
  pushNotification(entry).catch((err) =>
    console.warn("[dlssync] push system-driver notification failed:", err),
  );
}

export const currentView: Writable<string> = writable("library");

export const games: Writable<DetectedGame[]> = writable([]);
export const scanInProgress: Writable<boolean> = writable(false);

export const backups: Writable<BackupEntry[]> = writable([]);

export interface CatalogFamily {
  family: string;
  label: string;
  latest: string;
  releaseCount: number;
}
export interface CatalogVendor {
  vendor: string;
  label: string;
  families: CatalogFamily[];
}
export const catalogVendors: Writable<CatalogVendor[]> = writable([]);
export const catalogLatestByKey: Writable<Record<string, string>> = writable({});
export const catalogLatestShas: Writable<Record<string, string>> = writable({});

export { catalogShaKey } from "./relation";

export type CatalogChip = { kind: "accent" | "success" | "warning" | "danger"; label: string };
export const catalogStatus: Writable<CatalogChip> = writable({ kind: "accent", label: "idle" });
export const manifestUpdatedAt: Writable<string> = writable("");

export const settings: Writable<AppSettings | null> = writable(null);

export const gameDlls: Writable<Record<string, DllRecord[]>> = writable({});
export const gameDllsLoading: Writable<Record<string, boolean>> = writable({});
export const gameDllErrors: Writable<Record<string, string | null>> = writable({});
export const gameDlssEnabler: Writable<Record<string, boolean>> = writable({});

export interface ToastAction {
  label: string;
  run: () => void;
}
export interface Toast {
  id: number;
  kind: "success" | "warning" | "danger" | "info";
  message: string;
  action?: ToastAction;
  ttlMs: number;
}
export const toasts: Writable<Toast[]> = writable([]);
let toastSeq = 0;

export function showToast(kind: Toast["kind"], message: string, ttlMs = 4000): void {
  const id = ++toastSeq;
  toasts.update((arr) => [...arr, { id, kind, message, ttlMs }]);
  setTimeout(() => dismissToast(id), ttlMs);
}

/** A quiet toast carrying a single inline action (e.g. Undo). Returns the id so
 *  the caller can dismiss it early once the action is no longer reversible. */
export function showActionToast(
  kind: Toast["kind"],
  message: string,
  action: ToastAction,
  ttlMs = 6000,
): number {
  const id = ++toastSeq;
  toasts.update((arr) => [...arr, { id, kind, message, action, ttlMs }]);
  setTimeout(() => dismissToast(id), ttlMs);
  return id;
}

export function dismissToast(id: number): void {
  toasts.update((arr) => arr.filter((t) => t.id !== id));
}

/** Run a reversible toggle optimistically: flip UI state now, fire the backend
 *  call, and surface a quiet Undo toast. The optimistic state is reverted if the
 *  backend rejects OR the user clicks Undo; on success the toast lingers briefly
 *  with the Undo affordance, then auto-clears. */
export async function optimisticToggle(opts: {
  applyOptimistic: () => void;
  revert: () => void;
  commit: () => Promise<void>;
  message: string;
  undoLabel?: string;
}): Promise<void> {
  opts.applyOptimistic();
  let reverted = false;
  const undo = (): void => {
    if (reverted) return;
    reverted = true;
    opts.revert();
  };
  const toastId = showActionToast("info", opts.message, {
    label: opts.undoLabel ?? translate(get(locale), "common.undo"),
    run: () => {
      undo();
      dismissToast(toastId);
    },
  });
  try {
    await opts.commit();
  } catch (err: unknown) {
    if (!reverted) {
      undo();
      dismissToast(toastId);
      showToast("danger", translate(get(locale), "toast.saveFailed", { msg: formatError(err) }));
    }
  }
}

export const searchQuery: Writable<string> = writable("");
export const launcherFilter: Writable<string> = writable("all");
export type StatusFilter = UpdateStatus | "all" | "hidden";
export const statusFilter: Writable<StatusFilter> = writable("all");

export const drawerGameId: Writable<string | null> = writable(null);

export interface ApplyTracker {
  apply_id: string;
  group_id: string;
  game_id: string;
  game_label?: string;
  dll_path: string;
  family: string;
  target_version: string;
  stage: string;
  failed_at_stage: string | null;
  message: string;
  progress: number | null;
  error: string | null;
  error_class: string | null;
  attempt: number | null;
  bytes_downloaded: number;
  bytes_total: number | null;
  bytes_per_sec: number;
  started_at: number;
  ended_at: number | null;
}
export const activeApplies: Writable<Record<string, ApplyTracker>> = writable({});

export const applyModalOpen: Writable<boolean> = writable(false);

export interface GroupDownloadState {
  group_id: string;
  url: string;
  bytes_downloaded: number;
  bytes_total: number | null;
  bytes_per_sec: number;
  attempt: number;
  last_update: number;
}
export const downloadProgressByGroup: Writable<Record<string, GroupDownloadState>> = writable({});

export const inflightCount: Writable<number> = writable(0);

export type GameStatusMap = Record<string, UpdateStatus>;

export const catalogShasByVendor: Readable<CatalogShasByVendor> = derived(
  catalogLatestShas,
  ($shas) => buildShasByVendor($shas),
);

export const relationContext: Readable<RelationContext> = derived(
  [catalogLatestByKey, catalogLatestShas, catalogShasByVendor],
  ([$latestByKey, $shas, $shasByVendor]) => ({ latestByKey: $latestByKey, shas: $shas, shasByVendor: $shasByVendor }),
);

export const gameStatuses: Readable<GameStatusMap> = derived(
  [games, gameDlls, gameDllErrors, relationContext, settings],
  ([$games, $dlls, $errs, $ctx, $settings]) => {
    const out: GameStatusMap = {};
    const prefs = $settings?.update_prefs ?? null;
    for (const g of $games) {
      const disabled = $settings?.game_preferences[g.id]?.disabled_families ?? [];
      out[g.id] = gameStatusFromRecords($dlls[g.id], $ctx, disabled, $errs[g.id] ?? null, prefs);
    }
    return out;
  },
);

export const hiddenIds: Readable<Set<string>> = derived(settings, ($s) => new Set($s?.blacklist ?? []));

export const filteredGames: Readable<DetectedGame[]> = derived(
  [games, searchQuery, launcherFilter, statusFilter, gameStatuses, hiddenIds],
  ([$games, $q, $launcher, $status, $statuses, $hidden]) => {
    const q = $q.trim().toLowerCase();
    return $games.filter((g) => {
      const isHidden = $hidden.has(g.id);
      if ($status === "hidden") {
        if (!isHidden) return false;
      } else {
        if (isHidden) return false;
        if ($status !== "all" && $statuses[g.id] !== $status) return false;
      }
      if ($launcher !== "all" && g.launcher !== $launcher) return false;
      if (q && !g.name.toLowerCase().includes(q)) return false;
      return true;
    });
  },
);

const STATUS_SORT_RANK: Record<UpdateStatus, number> = {
  outdated: 0,
  up_to_date: 1,
  scanning: 2,
  unknown: 3,
  scan_failed: 4,
  no_dlls: 5,
};

export type LibraryZones = { actionable: DetectedGame[]; noDlls: DetectedGame[] };

export const libraryZones: Readable<LibraryZones> = derived(
  [filteredGames, gameStatuses],
  ([$filtered, $statuses]) => {
    const actionable: DetectedGame[] = [];
    const noDlls: DetectedGame[] = [];
    for (const g of $filtered) {
      const s: UpdateStatus = ($statuses[g.id] ?? "unknown") as UpdateStatus;
      if (s === "no_dlls") noDlls.push(g);
      else actionable.push(g);
    }
    const rank = (g: DetectedGame): number => STATUS_SORT_RANK[($statuses[g.id] ?? "unknown") as UpdateStatus] ?? 99;
    actionable.sort((a, b) => {
      const d = rank(a) - rank(b);
      return d !== 0 ? d : a.name.localeCompare(b.name);
    });
    noDlls.sort((a, b) => a.name.localeCompare(b.name));
    return { actionable, noDlls };
  },
);

export const outdatedGameCount: Readable<number> = derived(
  [games, gameStatuses, hiddenIds],
  ([$games, $statuses, $hidden]) =>
    $games.reduce((n, g) => (!$hidden.has(g.id) && $statuses[g.id] === "outdated" ? n + 1 : n), 0),
);

export const restorableBackupCount: Readable<number> = derived(
  backups,
  ($entries) => $entries.reduce((n, b) => (b.restored_at == null ? n + 1 : n), 0),
);

export interface SidebarCounts {
  library: number;
  backups: number;
}
export const sidebarCounts: Readable<SidebarCounts> = derived(
  [outdatedGameCount, restorableBackupCount],
  ([$outdated, $restorable]) => ({ library: $outdated, backups: $restorable }),
);

export const commandPaletteOpen: Writable<boolean> = writable(false);
export const notificationsOpen: Writable<boolean> = writable(false);
export const languageMenuOpen: Writable<boolean> = writable(false);
export const shortcutOverlayOpen: Writable<boolean> = writable(false);
/** Set by UpdateBanner while it occupies the bottom-left corner, so the support
 * card can yield and avoid overlapping it. */
export const updateBannerActive: Writable<boolean> = writable(false);
export const notificationsUnreadCount: Readable<number> = derived(
  notifications,
  ($n) => $n.filter((e) => e.read_at == null && e.dismissed_at == null).length,
);

export const requestThemeToggle: Writable<number> = writable(0);
export const requestApplyAllOutdated: Writable<number> = writable(0);
export const requestUpdateCheck: Writable<number> = writable(0);
export const requestRestoreMostRecent: Writable<number> = writable(0);

export function triggerThemeToggle(): void { requestThemeToggle.update((n) => n + 1); }
export function triggerApplyAllOutdated(): void { requestApplyAllOutdated.update((n) => n + 1); }
export function triggerUpdateCheck(): void { requestUpdateCheck.update((n) => n + 1); }
export function triggerRestoreMostRecent(): void { requestRestoreMostRecent.update((n) => n + 1); }


export async function scanGames(): Promise<void> {
  scanInProgress.set(true);
  try {
    const result = await scanLibraries();
    games.set(result);
    showToast("success", translate(get(locale), "toast.scanGamesFound", { count: result.length }));
    void loadAllDlls(result);
    void enrichManualArt(result);
  } catch (err: unknown) {
    const message = formatError(err);
    showToast("danger", translate(get(locale), "toast.scanFailed", { msg: message }));
    emitFailureNotification(
      "scan_failed",
      translate(get(locale), "notifKind.scan_failed"),
      message,
    );
  } finally {
    scanInProgress.set(false);
  }
}

const ART_PACING_MS = 150;

function firstArtUrl(art: GameArt): string | null {
  return art.grid_url ?? art.hero_url ?? art.capsule_url ?? null;
}

async function enrichManualArt(list: DetectedGame[]): Promise<void> {
  let apiKey = "";
  settings.subscribe((s) => { apiKey = (s?.steamgriddb.api_key ?? "").trim(); })();
  const targets = list.filter((g) => !g.image_url);
  for (const g of targets) {
    let url: string | null = null;
    try {
      url = firstArtUrl(await fetchSteamArt(g.name));
    } catch {
      url = null;
    }
    if (!url && apiKey) {
      try {
        url = firstArtUrl(await enrichGameArt(g.name, apiKey));
      } catch {
        url = null;
      }
    }
    if (url) {
      const resolved = url;
      games.update((arr) => arr.map((x) => (x.id === g.id ? { ...x, image_url: resolved } : x)));
    }
    await new Promise((r) => setTimeout(r, ART_PACING_MS));
  }
}

async function scanOne(g: DetectedGame): Promise<void> {
  gameDllsLoading.update((m) => ({ ...m, [g.id]: true }));
  let attempt = 0;
  let lastErr: unknown = null;
  while (attempt < 2) {
    try {
      const records = await detectDlls(g.install_dir);
      gameDlls.update((m) => ({ ...m, [g.id]: records }));
      gameDllErrors.update((m) => ({ ...m, [g.id]: null }));
      gameDllsLoading.update((m) => ({ ...m, [g.id]: false }));
      void detectDlssEnabler(g.install_dir)
        .then((flag) => gameDlssEnabler.update((m) => ({ ...m, [g.id]: flag })))
        .catch(() => {});
      return;
    } catch (err) {
      lastErr = err;
      attempt += 1;
      if (attempt < 2) await new Promise((r) => setTimeout(r, 250 + Math.floor(Math.random() * 200)));
    }
  }
  gameDllErrors.update((m) => ({ ...m, [g.id]: formatError(lastErr) }));
  gameDllsLoading.update((m) => ({ ...m, [g.id]: false }));
}

let failureToastShown = false;
async function loadAllDlls(list: DetectedGame[]): Promise<void> {
  const concurrency = 2;
  let idx = 0;
  failureToastShown = false;
  const workers: Promise<void>[] = [];
  for (let w = 0; w < concurrency; w++) {
    workers.push(
      (async () => {
        while (true) {
          const i = idx++;
          if (i >= list.length) return;
          await scanOne(list[i]);
        }
      })(),
    );
  }
  await Promise.all(workers);
  let errCount = 0;
  gameDllErrors.subscribe((m) => { errCount = Object.values(m).filter(Boolean).length; })();
  if (errCount > 0 && !failureToastShown) {
    failureToastShown = true;
    showToast("warning", translate(get(locale), "toast.gamesFailedToScan", { count: errCount }));
  }
}

export async function rescanGame(gameId: string): Promise<void> {
  let target: DetectedGame | undefined;
  games.subscribe((list) => { target = list.find((g) => g.id === gameId); })();
  if (!target) return;
  await scanOne(target);
}

function applySummary(summary: Awaited<ReturnType<typeof catalogSummary>>): void {
  const view: CatalogVendor[] = summary.vendors.map((v) => ({
    vendor: v.vendor,
    label: vendorLabel(v.vendor),
    families: v.families.map((f) => ({
      family: f.family,
      label: familyLabel(f.family),
      latest: f.latest,
      releaseCount: f.release_count,
    })),
  }));
  catalogVendors.set(view);
  const flat: Record<string, string> = {};
  for (const v of summary.vendors) {
    for (const f of v.families) {
      flat[f.family] = f.latest;
    }
  }
  catalogLatestByKey.set(flat);
  manifestUpdatedAt.set(new Date(summary.generated_at).toISOString().slice(0, 16).replace("T", " "));
}

export async function bootstrapCatalog(): Promise<void> {
  try {
    const summary = await catalogSummary();
    applySummary(summary);
    await loadCatalogShas();
    catalogStatus.set({ kind: "success", label: "ready" });
  } catch {
    catalogStatus.set({ kind: "warning", label: "loading" });
    void loadCatalog();
  }
}

export async function loadCatalog(): Promise<void> {
  catalogStatus.set({ kind: "warning", label: "loading" });
  const before = { ...get(catalogLatestByKey) };
  try {
    await refreshCatalog();
    const summary = await catalogSummary();
    applySummary(summary);
    await loadCatalogShas();
    catalogStatus.set({ kind: "success", label: "ready" });
    const after = get(catalogLatestByKey);
    const diffs = diffCatalogLatest(before, after);
    if (diffs.length > 0) emitCatalogUpdateNotifications(diffs);
  } catch (err: unknown) {
    catalogStatus.set({ kind: "danger", label: "error" });
    const message = formatError(err);
    showToast("danger", translate(get(locale), "toast.catalogRefreshFailed", { msg: message }));
    emitFailureNotification(
      "catalog_refresh_failed",
      translate(get(locale), "notifKind.catalog_refresh_failed"),
      message,
    );
  }
}

async function loadCatalogShas(): Promise<void> {
  try {
    const shas = await fetchCatalogLatestShas();
    catalogLatestShas.set(shas);
  } catch (err: unknown) {
    console.warn("[dlssync] catalog SHAs unavailable:", err);
  }
}

export async function loadBackups(): Promise<void> {
  try {
    const result = await listBackups();
    backups.set(result);
  } catch (err: unknown) {
    showToast("warning", translate(get(locale), "toast.backupsLoadFailed", { msg: formatError(err) }));
  }
}

export const driverReports: Writable<DriverStatusReport[]> = writable([]);
export const driverCheckInProgress: Writable<boolean> = writable(false);
export const driverCheckError: Writable<string | null> = writable(null);

export async function loadDriverUpdates(): Promise<void> {
  driverCheckInProgress.set(true);
  driverCheckError.set(null);
  try {
    const reports = await checkDriverUpdates();
    driverReports.set(sortDriverReports(reports));
    emitDriverUpdateNotifications(reports);
  } catch (err: unknown) {
    const message = formatError(err);
    driverCheckError.set(message);
    showToast("danger", translate(get(locale), "toast.driverCheckFailed", { msg: message }));
  } finally {
    driverCheckInProgress.set(false);
  }
}

/** Shared, app-level install progress for the GPU-driver updater. Lives in the
 *  store (not in the Drivers view) so it survives unmounting when the user
 *  switches tabs mid-download — the app-level listener in `driverInstallEvents`
 *  keeps writing here regardless of which view is mounted. */
export interface DriverInstallState {
  vendor: GpuVendor | null;
  stage: InstallStage | null;
  message: string;
  fraction: number | null;
}

const DRIVER_INSTALL_IDLE: DriverInstallState = {
  vendor: null,
  stage: null,
  message: "",
  fraction: null,
};

export const driverInstall: Writable<DriverInstallState> = writable({ ...DRIVER_INSTALL_IDLE });

/** Fold a backend progress event into the shared state. Ignored when no install
 *  is active so a late event cannot resurrect a cleared card. */
export function applyDriverInstallProgress(p: DriverInstallProgress): void {
  driverInstall.update((s) =>
    s.vendor === null ? s : { ...s, stage: p.stage, message: p.message, fraction: p.progress },
  );
}

/** Drive a driver install end-to-end. One install at a time; progress is
 *  reflected in the shared `driverInstall` store and cleared on completion. */
export async function startDriverInstall(report: DriverStatusReport): Promise<void> {
  const url = report.latest?.download_url;
  if (!url || get(driverInstall).vendor) return;
  driverInstall.set({
    vendor: report.device.vendor,
    stage: "downloading",
    message: translate(get(locale), "toast.starting"),
    fraction: null,
  });
  try {
    const outcome = await installDriver(report.device.vendor, url);
    if (outcome.stage === "completed") {
      showToast("success", outcome.message);
      await loadDriverUpdates();
    } else if (outcome.stage === "cancelled") {
      showToast("warning", outcome.message);
    } else {
      showToast("danger", outcome.message);
    }
  } catch (err: unknown) {
    showToast("danger", translate(get(locale), "toast.installFailed", { msg: formatError(err) }));
  } finally {
    driverInstall.set({ ...DRIVER_INSTALL_IDLE });
  }
}

/** Cache of historical driver lists per GPU model, lazy-loaded on first
 *  flyout open and reused across opens within the session. */
export const driverHistory: Writable<Record<string, DriverReleaseDto[]>> = writable({});
export const driverHistoryLoading: Writable<Record<string, boolean>> = writable({});

export async function loadDriverHistory(
  model: string,
  vendor: "nvidia" | "amd" | "intel",
): Promise<void> {
  driverHistoryLoading.update((m) => ({ ...m, [model]: true }));
  try {
    const releases = await listDriverHistory(model, vendor);
    driverHistory.update((m) => ({ ...m, [model]: releases }));
  } catch (err: unknown) {
    showToast("warning", translate(get(locale), "toast.driverHistoryLoadFailed", { msg: formatError(err) }));
  } finally {
    driverHistoryLoading.update((m) => ({ ...m, [model]: false }));
  }
}


export const systemDriverGroups: Writable<SystemDeviceGroup[]> = writable([]);
export const systemScanInProgress: Writable<boolean> = writable(false);
export const systemScanError: Writable<string | null> = writable(null);
export const systemScanRan: Writable<boolean> = writable(false);

export async function loadSystemDrivers(): Promise<void> {
  systemScanInProgress.set(true);
  systemScanError.set(null);
  try {
    const groups = await scanSystemDrivers();
    systemDriverGroups.set(groups);
    systemScanRan.set(true);
    emitSystemDriverUpdateNotification(groups);
  } catch (err: unknown) {
    const message = formatError(err);
    systemScanError.set(message);
    showToast("danger", translate(get(locale), "toast.systemDriverScanFailed", { msg: message }));
  } finally {
    systemScanInProgress.set(false);
  }
}

/** Shared, app-level install state for a single system-driver update. Keyed by
 *  the WUA `update_id` so the card that launched it shows progress regardless of
 *  which view is mounted (the app-level listener keeps writing here). */
export interface SystemDriverInstallState {
  updateId: string | null;
  stage: SystemDriverProgress["stage"] | null;
  message: string;
  fraction: number | null;
}

const SYSTEM_DRIVER_IDLE: SystemDriverInstallState = {
  updateId: null,
  stage: null,
  message: "",
  fraction: null,
};

/** Human label per install stage (the backend enum is machine-cased). */
export const DRIVER_INSTALL_STAGE_LABEL: Record<string, string> = {
  downloading: "Downloading",
  installing: "Installing",
  completed: "Done",
  failed: "Failed",
};

export const systemDriverInstall: Writable<SystemDriverInstallState> = writable({
  ...SYSTEM_DRIVER_IDLE,
});

/** Fold a backend progress event into the shared state. Ignored when no install
 *  is active so a late event cannot resurrect a cleared card. */
export function applySystemDriverProgress(p: SystemDriverProgress): void {
  systemDriverInstall.update((s) =>
    s.updateId === null ? s : { ...s, stage: p.stage, message: p.message, fraction: p.fraction },
  );
}

/** Drive a system-driver install end-to-end. One at a time; on success the
 *  scan is refreshed so the freshly-installed driver drops off the list. */
export async function startSystemDriverInstall(
  update: SystemDriverUpdate,
  deviceClassLabel?: string,
): Promise<void> {
  if (get(systemDriverInstall).updateId) return;
  systemDriverInstall.set({
    updateId: update.update_id,
    stage: "downloading",
    message: translate(get(locale), "toast.starting"),
    fraction: null,
  });
  try {
    const outcome = await installSystemDriver(
      update.update_id,
      driverInstallContext(update, deviceClassLabel ?? update.class),
    );
    if (outcome.success) {
      const loc = get(locale);
      const reboot = outcome.reboot_required ? translate(loc, "toast.systemDriverInstalledReboot") : "";
      showToast("success", translate(loc, "toast.systemDriverInstalled", { title: update.title, reboot }));
      systemDriverInstall.set({ ...SYSTEM_DRIVER_IDLE });
      await loadSystemDrivers();
    } else {
      showToast("danger", outcome.message);
      failSystemDriverInstall(update.update_id, outcome.message);
    }
  } catch (err: unknown) {
    const message = translate(get(locale), "toast.installFailed", { msg: formatError(err) });
    showToast("danger", message);
    failSystemDriverInstall(update.update_id, message);
  }
}

/** Park the card in a VISIBLE terminal 'failed' state (instead of silently
 *  snapping back to idle), then auto-clear after a grace period. */
function failSystemDriverInstall(id: string, message: string): void {
  systemDriverInstall.set({ updateId: id, stage: "failed", message, fraction: 1 });
  setTimeout(() => {
    const s = get(systemDriverInstall);
    if (s.updateId === id && s.stage === "failed") {
      systemDriverInstall.set({ ...SYSTEM_DRIVER_IDLE });
    }
  }, 8000);
}

/** Manually clear a finished/failed install card. */
export function dismissSystemDriverInstall(): void {
  systemDriverInstall.set({ ...SYSTEM_DRIVER_IDLE });
}

export interface DockItem {
  id: string;
  kind: "apply" | "gpu_driver" | "system_driver";
  label: string;
  stage: string;
  fraction: number | null;
}

export const dockItems: Readable<DockItem[]> = derived(
  [activeApplies, driverInstall, systemDriverInstall],
  ([$applies, $gpu, $sys]) => {
    const items: DockItem[] = [];
    for (const a of Object.values($applies)) {
      if (a.ended_at !== null) continue;
      items.push({
        id: a.apply_id,
        kind: "apply",
        label: a.game_label ?? a.game_id,
        stage: a.stage,
        fraction: a.progress,
      });
    }
    if ($gpu.vendor !== null) {
      items.push({
        id: `gpu:${$gpu.vendor}`,
        kind: "gpu_driver",
        label: `${$gpu.vendor.toUpperCase()} driver`,
        stage: $gpu.stage ?? "",
        fraction: $gpu.fraction,
      });
    }
    if ($sys.updateId !== null) {
      items.push({
        id: `sys:${$sys.updateId}`,
        kind: "system_driver",
        label: "Component driver",
        stage: $sys.stage ?? "",
        fraction: $sys.fraction,
      });
    }
    return items;
  },
);

export async function loadSettings(): Promise<void> {
  try {
    const result = await getSettings();
    settings.set(result);
  } catch (err: unknown) {
    showToast("danger", translate(get(locale), "toast.settingsLoadFailed", { msg: formatError(err) }));
  }
}

export async function persistSettings(next: AppSettings): Promise<void> {
  try {
    await saveSettings(next);
    settings.set(next);
  } catch (err: unknown) {
    showToast("danger", translate(get(locale), "toast.settingsSaveFailed", { msg: formatError(err) }));
  }
}

function formatError(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}
