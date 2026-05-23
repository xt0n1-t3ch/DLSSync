import { invoke } from "@tauri-apps/api/core";

export type LauncherKind =
  | "steam"
  | "epic"
  | "gog"
  | "ubisoft"
  | "ea_desktop"
  | "xbox"
  | "battlenet"
  | "manual";

export interface DetectedGame {
  id: string;
  name: string;
  launcher: LauncherKind;
  install_dir: string;
  app_id: string | null;
  image_url: string | null;
  size_bytes: number | null;
}

export type UpdateStatus = "outdated" | "up_to_date" | "no_dlls" | "unknown" | "scanning";

export type DllFamily =
  | "dlss_sr"
  | "dlss_fg"
  | "dlss_rr"
  | "streamline"
  | "streamline_common"
  | "streamline_pcl"
  | "streamline_nis"
  | "streamline_direct_sr"
  | "reflex"
  | "xess_sr"
  | "xess_sr_dx11"
  | "xess_fg"
  | "xell"
  | "fsr_upscaler"
  | "fsr_upscaler_vk"
  | "fsr_fg"
  | "fsr_loader"
  | "direct_storage";

export interface DllRecord {
  family: DllFamily;
  path: string;
  current_version: string | null;
  file_description: string | null;
  sha256: string | null;
}

export interface Release {
  version: string;
  version_packed: number;
  filename: string;
  sha256: string;
  size_bytes: number;
  signed: boolean;
  released_at: string;
  source: string;
  cdn_url: string;
  release_notes: string | null;
  signature_subject: string | null;
  channel: "stable" | "experimental" | string;
  is_dev: boolean;
  min_driver: string | null;
}

export interface BackupEntry {
  id: string;
  game_id: string;
  dll_family: string;
  dll_filename: string;
  original_path: string;
  backup_path: string;
  previous_version: string | null;
  previous_sha256: string | null;
  created_at: string;
  restored_at: string | null;
  size_bytes: number | null;
}

export interface DeleteOutcome {
  removed_file: boolean;
  removed_empty_dirs: number;
  file_error: string | null;
}

export interface AppPathsDto {
  root: string;
  backups_dir: string;
  cache_dir: string;
  logs_dir: string;
  settings_dir: string;
  backups_db: string;
  catalog_cache: string;
  settings_file: string;
}

export type GpuVendor = "nvidia" | "amd" | "intel" | "other";

export interface OsInfo {
  name: string;
  version: string;
  build: string;
  edition: string;
}

export interface CpuInfo {
  brand: string;
  physical_cores: number;
  logical_cores: number;
}

export interface RamModule {
  capacity_bytes: number;
  mhz: number;
  type_label: string;
}

export interface RamInfo {
  total_bytes: number;
  modules: RamModule[];
}

export interface GpuInfo {
  vendor: GpuVendor;
  model: string;
  driver_version: string;
  vram_bytes: number;
  recommended_runtimes: string[];
}

export interface SystemInfo {
  os: OsInfo;
  cpu: CpuInfo;
  ram: RamInfo;
  gpus: GpuInfo[];
  collected_at: string;
}

export interface CatalogSummary {
  generated_at: string;
  vendors: VendorSummary[];
  incompatible_games: string[];
}

export interface VendorSummary {
  vendor: string;
  families: FamilySummary[];
}

export interface FamilySummary {
  family: string;
  latest: string;
  release_count: number;
}

export interface LauncherOverrides {
  steam: string[];
  epic: string[];
  gog: string[];
  ubisoft: string[];
  ea_desktop: string[];
  xbox: string[];
  battlenet: string[];
  custom: string[];
}

export interface UpdatePreferences {
  update_dlss: boolean;
  update_dlss_fg: boolean;
  update_dlss_rr: boolean;
  update_streamline: boolean;
  update_reflex: boolean;
  update_xess: boolean;
  update_fsr: boolean;
  update_direct_storage: boolean;
  create_backups: boolean;
  auto_apply_all_on_rescan: boolean;
}

export interface UiPreferences {
  theme: string;
  sidebar_collapsed: boolean;
  grid_density: string;
  sort_order: string;
  launcher_filter: string;
  status_filter: string;
}

export interface SteamApiConfig {
  api_key: string;
  steam_id: string;
}

export interface SgdbConfig {
  api_key: string;
}

export interface GameArt {
  grid_url: string | null;
  hero_url: string | null;
  capsule_url: string | null;
}

export interface WindowState {
  width: number | null;
  height: number | null;
  top: number | null;
  left: number | null;
  maximized: boolean;
}

export interface GamePreference {
  disabled_families: string[];
  pinned_versions: Record<string, string>;
}

export interface AdvancedConfig {
  dlss_debug_overlay: boolean;
  verbose_logs: boolean;
  allow_unsigned_dlls: boolean;
  prefer_stable_channel: boolean;
}

export interface AppSettings {
  launcher_overrides: LauncherOverrides;
  update_prefs: UpdatePreferences;
  ui_prefs: UiPreferences;
  steam_api: SteamApiConfig;
  steamgriddb: SgdbConfig;
  window_state: WindowState;
  blacklist: string[];
  ignored: string[];
  game_preferences: Record<string, GamePreference>;
  advanced: AdvancedConfig;
}

export interface ApplyRequest {
  apply_id: string;
  game_id: string;
  dll_path: string;
  vendor: string;
  family: string;
  target_version: string;
  game_label?: string;
}

export interface ApplyResult {
  apply_id: string;
  backup_id: string;
  previous_version: string | null;
  new_version: string;
}

export interface ApplyProgress {
  apply_id: string;
  stage: "download" | "verify_sha" | "verify_signature" | "backup" | "replace" | "verify_post" | "complete" | "failed";
  message: string;
  progress: number | null;
  error: string | null;
}

export const DEFAULT_LAUNCHERS: LauncherKind[] = [
  "steam",
  "epic",
  "gog",
  "ubisoft",
  "ea_desktop",
  "xbox",
  "battlenet",
];

export async function scanLibraries(
  launchers: LauncherKind[] = DEFAULT_LAUNCHERS,
): Promise<DetectedGame[]> {
  return invoke("scan_libraries", { launchers });
}

export async function detectDlls(installDir: string): Promise<DllRecord[]> {
  return invoke("detect_dlls", { installDir });
}

export async function refreshCatalog(): Promise<void> {
  return invoke("refresh_catalog");
}

export async function catalogSummary(): Promise<CatalogSummary> {
  return invoke("catalog_summary");
}

export async function catalogLatestShas(): Promise<Record<string, string>> {
  return invoke("catalog_latest_shas");
}

export async function listReleases(vendor: string, family: string): Promise<Release[]> {
  return invoke("list_releases", { vendor, family });
}

export async function listBackups(): Promise<BackupEntry[]> {
  return invoke("list_backups");
}

export async function restoreBackup(backupId: string): Promise<void> {
  return invoke("restore_backup", { backupId });
}

export async function deleteBackup(backupId: string): Promise<DeleteOutcome> {
  return invoke("delete_backup", { backupId });
}

export async function getAppPaths(): Promise<AppPathsDto> {
  return invoke("get_app_paths");
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke("get_system_info");
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function addBlacklistEntry(gameId: string): Promise<string[]> {
  return invoke("add_blacklist_entry", { gameId });
}

export async function removeBlacklistEntry(gameId: string): Promise<string[]> {
  return invoke("remove_blacklist_entry", { gameId });
}

export async function saveWindowState(windowState: WindowState): Promise<void> {
  return invoke("save_window_state", { windowState });
}

export async function applyUpdate(request: ApplyRequest): Promise<ApplyResult> {
  return invoke("apply_update", { request });
}

export async function setDlssDebugOverlay(enabled: boolean): Promise<void> {
  return invoke("set_dlss_debug_overlay", { enabled });
}

export async function getDlssDebugOverlay(): Promise<boolean> {
  return invoke("get_dlss_debug_overlay");
}

export async function enrichGameArt(name: string, apiKey: string): Promise<GameArt> {
  return invoke("enrich_game_art", { name, apiKey });
}

export async function fetchSteamArt(name: string): Promise<GameArt> {
  return invoke("fetch_steam_art", { name });
}

export async function openPath(path: string): Promise<void> {
  return invoke("open_path", { path });
}

export async function revealPath(path: string): Promise<void> {
  return invoke("reveal_path", { path });
}

export async function setCloseToTray(enable: boolean): Promise<void> {
  return invoke("set_close_to_tray", { enable });
}

export async function getCloseToTray(): Promise<boolean> {
  return invoke("get_close_to_tray");
}

export async function setEfficiencyMode(enable: boolean): Promise<void> {
  return invoke("set_efficiency_mode", { enable });
}

export async function hideMainWindow(): Promise<void> {
  return invoke("hide_main_window");
}

export async function showMainWindow(): Promise<void> {
  return invoke("show_main_window");
}
