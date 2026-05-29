import type { DllRecord, UpdatePreferences } from "./api";
import { familyVendor, familyCatalogKey, filenameFromPath, type UpdateStatus } from "./labels";

const FAMILY_PREF: Record<string, keyof UpdatePreferences> = {
  dlss_sr: "update_dlss",
  dlss_fg: "update_dlss_fg",
  dlss_rr: "update_dlss_rr",
  streamline: "update_streamline",
  streamline_common: "update_streamline",
  streamline_pcl: "update_streamline",
  streamline_nis: "update_streamline",
  streamline_direct_sr: "update_streamline",
  reflex: "update_reflex",
  xess_sr: "update_xess",
  xess_sr_dx11: "update_xess",
  xess_fg: "update_xess",
  xell: "update_xess",
  fsr_upscaler: "update_fsr",
  fsr_upscaler_vk: "update_fsr",
  fsr_fg: "update_fsr",
  fsr_loader: "update_fsr",
  fsr_denoiser: "update_fsr",
  direct_storage: "update_direct_storage",
  direct_storage_core: "update_direct_storage",
};

/** An NVIDIA Streamline plugin/interposer DLL (`sl.*.dll`) — a version-locked set
 * that must not be swapped piecemeal (mirrors the backend `is_streamline_plugin`). */
export function isStreamlinePlugin(filename: string): boolean {
  const lower = filename.toLowerCase();
  return lower.startsWith("sl.") && lower.endsWith(".dll");
}

/** Whether a DLL may be offered/applied given the user's per-feature preferences.
 * `sl.*` Streamline plugins additionally require the Streamline master switch and
 * are never offered when DLSS Enabler manages the Streamline set in this game —
 * swapping them out of lockstep crashes the game (kFeatureReflex). `prefs === null`
 * keeps the legacy permissive behaviour. */
export function recordUpdatable(
  r: DllRecord,
  prefs: UpdatePreferences | null = null,
  dlssEnablerPresent = false,
): boolean {
  const streamlinePlugin = isStreamlinePlugin(filenameFromPath(r.path));
  if (streamlinePlugin && dlssEnablerPresent) return false;
  if (!prefs) return true;
  const prefKey = FAMILY_PREF[r.family];
  if (prefKey && !prefs[prefKey]) return false;
  if (streamlinePlugin && !prefs.update_streamline) return false;
  return true;
}

export type DllRelation = "outdated" | "same" | "ahead" | "no-target";

export type CatalogShasByVendor = Record<string, Set<string>>;

export type RelationContext = {
  latestByKey: Record<string, string>;
  shas: Record<string, string>;
  shasByVendor: CatalogShasByVendor;
};

export function catalogShaKey(vendor: string, family: string, filename: string): string {
  return `${vendor.toLowerCase()}::${family.toLowerCase()}::${filename.toLowerCase()}`;
}

export function buildShasByVendor(shas: Record<string, string>): CatalogShasByVendor {
  const map: CatalogShasByVendor = {};
  for (const [key, sha] of Object.entries(shas)) {
    const vendor = key.split("::")[0];
    if (!map[vendor]) map[vendor] = new Set();
    map[vendor].add(sha.toLowerCase());
  }
  return map;
}

export function targetVersion(r: DllRecord, ctx: RelationContext, pinnedVersion: string | null = null): string | null {
  if (pinnedVersion) return pinnedVersion;
  return ctx.latestByKey[familyCatalogKey(r.family)] ?? null;
}

function compareVersionString(a: string, b: string): number {
  const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const x = pa[i] ?? 0;
    const y = pb[i] ?? 0;
    if (x < y) return -1;
    if (x > y) return 1;
  }
  return 0;
}

export function dllRelation(r: DllRecord, ctx: RelationContext, pinnedVersion: string | null = null): DllRelation {
  const target = targetVersion(r, ctx, pinnedVersion);
  if (!target) return "no-target";
  const vendor = familyVendor(r.family);
  const family = familyCatalogKey(r.family);
  const filename = filenameFromPath(r.path);
  const expectedSha = ctx.shas[catalogShaKey(vendor, family, filename)];
  const installedSha = r.sha256?.toLowerCase() ?? null;
  if (expectedSha && installedSha && installedSha === expectedSha.toLowerCase()) {
    return "same";
  }
  if (installedSha && ctx.shasByVendor[vendor]?.has(installedSha)) {
    return "same";
  }
  if (!r.current_version) return "no-target";
  const c = compareVersionString(r.current_version, target);
  if (c < 0) return "outdated";
  if (c > 0) return "ahead";
  return "same";
}

export function isOutdated(r: DllRecord, ctx: RelationContext, pinnedVersion: string | null = null): boolean {
  return dllRelation(r, ctx, pinnedVersion) === "outdated";
}

export function gameStatusFromRecords(
  records: DllRecord[] | undefined,
  ctx: RelationContext,
  disabledFamilies: string[] = [],
  scanError: string | null = null,
  prefs: UpdatePreferences | null = null,
  dlssEnablerPresent = false,
): UpdateStatus {
  if (scanError) return "scan_failed";
  if (!records) return "unknown";
  if (records.length === 0) return "no_dlls";
  for (const r of records) {
    if (disabledFamilies.includes(r.family)) continue;
    if (!recordUpdatable(r, prefs, dlssEnablerPresent)) continue;
    if (dllRelation(r, ctx) === "outdated") return "outdated";
  }
  return "up_to_date";
}
