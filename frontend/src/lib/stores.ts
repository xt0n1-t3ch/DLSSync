import { writable, derived, type Writable, type Readable } from "svelte/store";
import {
  scanLibraries,
  refreshCatalog,
  catalogSummary,
  catalogLatestShas as fetchCatalogLatestShas,
  listBackups,
  getSettings,
  saveSettings,
  detectDlls,
  enrichGameArt,
  fetchSteamArt,
  type DetectedGame,
  type BackupEntry,
  type AppSettings,
  type DllRecord,
  type GameArt,
} from "./api";
import { vendorLabel, familyLabel, type UpdateStatus } from "./labels";
import { buildShasByVendor, gameStatusFromRecords, type CatalogShasByVendor, type RelationContext } from "./relation";

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

export interface Toast {
  id: number;
  kind: "success" | "warning" | "danger" | "info";
  message: string;
}
export const toasts: Writable<Toast[]> = writable([]);
let toastSeq = 0;

export function showToast(kind: Toast["kind"], message: string, ttlMs = 4000): void {
  const id = ++toastSeq;
  toasts.update((arr) => [...arr, { id, kind, message }]);
  setTimeout(() => dismissToast(id), ttlMs);
}

export function dismissToast(id: number): void {
  toasts.update((arr) => arr.filter((t) => t.id !== id));
}

export const searchQuery: Writable<string> = writable("");
export const launcherFilter: Writable<string> = writable("all");
export type StatusFilter = UpdateStatus | "all" | "hidden";
export const statusFilter: Writable<StatusFilter> = writable("all");

export const drawerGameId: Writable<string | null> = writable(null);

export interface ApplyTracker {
  apply_id: string;
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
}
export const activeApplies: Writable<Record<string, ApplyTracker>> = writable({});

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
    for (const g of $games) {
      const disabled = $settings?.game_preferences[g.id]?.disabled_families ?? [];
      out[g.id] = gameStatusFromRecords($dlls[g.id], $ctx, disabled, $errs[g.id] ?? null);
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


export async function scanGames(): Promise<void> {
  scanInProgress.set(true);
  try {
    const result = await scanLibraries();
    games.set(result);
    showToast("success", `Found ${result.length} games`);
    void loadAllDlls(result);
    void enrichManualArt(result);
  } catch (err: unknown) {
    showToast("danger", `Scan failed: ${formatError(err)}`);
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
    showToast("warning", `${errCount} game${errCount === 1 ? "" : "s"} failed to scan — open and Rescan to retry`);
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
  try {
    await refreshCatalog();
    const summary = await catalogSummary();
    applySummary(summary);
    await loadCatalogShas();
    catalogStatus.set({ kind: "success", label: "ready" });
  } catch (err: unknown) {
    catalogStatus.set({ kind: "danger", label: "error" });
    showToast("danger", `Catalog refresh failed: ${formatError(err)}`);
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
    showToast("warning", `Could not load backups: ${formatError(err)}`);
  }
}

export async function loadSettings(): Promise<void> {
  try {
    const result = await getSettings();
    settings.set(result);
  } catch (err: unknown) {
    showToast("danger", `Could not load settings: ${formatError(err)}`);
  }
}

export async function persistSettings(next: AppSettings): Promise<void> {
  try {
    await saveSettings(next);
    settings.set(next);
  } catch (err: unknown) {
    showToast("danger", `Could not save settings: ${formatError(err)}`);
  }
}

function formatError(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}
