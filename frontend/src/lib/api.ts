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

export type UpdateStatus =
  | "outdated"
  | "up_to_date"
  | "no_dlls"
  | "unknown"
  | "scanning"
  | "scan_failed";

export type DllFamily =
  | "dlss_sr"
  | "dlss_fg"
  | "dlss_rr"
  | "sl_dlss_sr"
  | "sl_dlss_fg"
  | "sl_dlss_rr"
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
  | "fsr_denoiser"
  | "direct_storage"
  | "direct_storage_core";

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
  backup_type: string;
  device_class: string | null;
  hardware_id: string | null;
  driver_provider: string | null;
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
  pci_vendor_id: number;
  pci_device_id: number;
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

export type DriverUpdateStatus = "up_to_date" | "update_available" | "unknown" | "unsupported";

export interface DriverVersionDto {
  packed: number;
  display: string;
  raw: string;
}

export interface DriverChangelogDto {
  highlights: string[];
  fixed: string[];
  notes_page_url: string | null;
}

export interface DriverReleaseDto {
  vendor: GpuVendor;
  version: DriverVersionDto;
  channel: "stable" | "beta";
  display_version: string | null;
  is_beta: boolean;
  download_url: string;
  size_bytes: number;
  signature_subject: string;
  released_at: string | null;
  release_notes_url: string | null;
  changelog: DriverChangelogDto | null;
}

export interface DeviceIdDto {
  class: "gpu";
  vendor: GpuVendor;
  pci_vendor_id: number;
  pci_device_id: number;
  model: string;
}

export interface DriverStatusReport {
  device: DeviceIdDto;
  installed: DriverVersionDto;
  latest: DriverReleaseDto | null;
  status: DriverUpdateStatus;
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
  library_view_mode: LibraryViewMode;
  library_density: LibraryDensity;
  library_sort: LibrarySort;
  backups_group_by: BackupsGroupBy;
  settings_active_tab: SettingsTab;
  command_palette_recent: string[];
  show_support_nudge: boolean;
  language: string;
}

export type LibraryViewMode = "grid" | "list";
export type LibraryDensity = "compact" | "comfy";
export type LibrarySort =
  | "default"
  | "outdated_first"
  | "recently_played"
  | "a_z"
  | "z_a"
  | "launcher";
export type BackupsGroupBy = "game" | "date";
export type SettingsTab = "general" | "updates" | "detection" | "art" | "advanced";

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
  apply_concurrency: number;
}

export interface NetworkConfig {
  retry_attempts: number;
  download_cache_ttl_secs: number;
  connect_timeout_secs: number;
  chunk_timeout_secs: number;
}

/** Background-scan daemon settings. Mirrors the Rust `BackgroundConfig`
 *  (commands/settings.rs) field-for-field in snake_case; every field is
 *  `serde(default)` on the Rust side so legacy settings.json migrates cleanly. */
export interface BackgroundConfig {
  enabled: boolean;
  /** Re-scan cadence; the scheduler clamps to 1..=168 when it reads it. */
  interval_hours: number;
  close_to_tray: boolean;
  run_at_startup: boolean;
  notify_os_toast: boolean;
  auto_apply: boolean;
}

export const BACKGROUND_INTERVAL_MIN_HOURS = 1;
export const BACKGROUND_INTERVAL_MAX_HOURS = 168;
export const BACKGROUND_INTERVAL_DEFAULT_HOURS = 24;

export const DEFAULT_BACKGROUND_CONFIG: BackgroundConfig = {
  enabled: false,
  interval_hours: BACKGROUND_INTERVAL_DEFAULT_HOURS,
  close_to_tray: false,
  run_at_startup: false,
  notify_os_toast: true,
  auto_apply: false,
};

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
  network: NetworkConfig;
  background: BackgroundConfig;
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

export type ApplyStage =
  | "download"
  | "verify_sha"
  | "verify_signature"
  | "backup"
  | "replace"
  | "verify_post"
  | "complete"
  | "failed"
  | "cancelled";

export type ApplyErrorClass =
  | "network"
  | "signature"
  | "lock"
  | "permission"
  | "hash"
  | "missing"
  | "backup"
  | "cancelled"
  | "other";

export interface ApplyProgress {
  apply_id: string;
  group_id: string;
  stage: ApplyStage;
  message: string;
  progress: number | null;
  error: string | null;
  error_class?: ApplyErrorClass | null;
  attempt?: number | null;
}

export interface GroupDownloadProgress {
  group_id: string;
  url: string;
  bytes_downloaded: number;
  bytes_total: number | null;
  bytes_per_sec: number;
  attempt: number;
}

export interface InflightSnapshot {
  in_flight: number;
}

export interface ApplyBatchRequest {
  items: ApplyRequest[];
}

export interface ApplyOutcome {
  apply_id: string;
  success: boolean;
  backup_id: string | null;
  previous_version: string | null;
  new_version: string | null;
  error: string | null;
}

export interface ApplyBatchResult {
  outcomes: ApplyOutcome[];
}

export interface StreamlineSetResult {
  success: boolean;
  applied: ApplyOutcome[];
  error: string | null;
  rolled_back: boolean;
}

export const APPLY_STAGES: { id: ApplyStage; label: string }[] = [
  { id: "download", label: "Download" },
  { id: "verify_sha", label: "Verify SHA" },
  { id: "verify_signature", label: "Verify signature" },
  { id: "backup", label: "Backup current" },
  { id: "replace", label: "Install new" },
  { id: "verify_post", label: "Verify installed" },
  { id: "complete", label: "Done" },
];

export const APPLY_PROGRESS_EVENT = "apply_progress";
export const DOWNLOAD_PROGRESS_EVENT = "download_progress";
export const APPLY_INFLIGHT_EVENT = "apply_inflight";
export const TRAY_CHECK_UPDATE_EVENT = "tray://check-update";
export const TRAY_SHOW_PROGRESS_EVENT = "tray://show-progress";

/** Backend -> frontend: the background scheduler fired a scan tick. */
export const BACKGROUND_SCAN_TICK_EVENT = "background:scan-tick";
/** Backend -> frontend (tray "Apply all updates"): run the Apply-All flow. */
export const BACKGROUND_APPLY_ALL_EVENT = "background:apply-all";

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

export async function detectDlssEnabler(installDir: string): Promise<boolean> {
  return invoke("detect_dlss_enabler", { installDir });
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

export async function checkDriverUpdates(): Promise<DriverStatusReport[]> {
  return invoke("check_driver_updates");
}

export async function listDriverHistory(
  model: string,
  vendor: "nvidia" | "amd" | "intel",
): Promise<DriverReleaseDto[]> {
  return invoke("list_driver_history", { model, vendor });
}

export type ProtectionKind = "anti_cheat" | "anti_tamper" | "drm";
export type ProtectionSource = "binary" | "pe" | "dataset";

export interface DetectedAntiCheat {
  anticheat: string;
  kind: ProtectionKind;
  source: ProtectionSource;
}

export interface AntiCheatReport {
  detected: DetectedAntiCheat[];
  status: string | null;
  source_url: string | null;
}

export async function detectAnticheat(
  installDir: string,
  appId: string | null,
  name: string,
): Promise<AntiCheatReport> {
  return invoke("detect_anticheat", { installDir, appId, name });
}

export type InstallStage =
  | "queued"
  | "downloading"
  | "verifying"
  | "launching"
  | "installing"
  | "completed"
  | "failed"
  | "cancelled";

export interface DriverInstallProgress {
  stage: InstallStage;
  message: string;
  progress: number | null;
}

export interface DriverInstallOutcome {
  stage: InstallStage;
  exit_code: number;
  message: string;
}

export const DRIVER_INSTALL_EVENT = "driver_install_progress";

export async function installDriver(
  vendor: string,
  downloadUrl: string,
): Promise<DriverInstallOutcome> {
  return invoke("install_driver", { vendor, downloadUrl });
}


export type SystemDeviceClass =
  | "audio"
  | "display"
  | "monitor"
  | "network"
  | "bluetooth"
  | "input"
  | "storage"
  | "printer"
  | "camera"
  | "sensor"
  | "battery"
  | "smart_card"
  | "firmware"
  | "chipset"
  | "system"
  | "usb"
  | "other";

export interface SystemDriverUpdate {
  update_id: string;
  title: string;
  class: SystemDeviceClass;
  provider: string;
  driver_version: string | null;
  driver_date: string | null;
  hardware_id: string | null;
  size_bytes: number;
  target_device: string | null;
  current_version: string | null;
  /** DriverStore `oemNN.inf` of the matched installed device, for snapshot + version history. */
  target_inf: string | null;
  /** Hardware id of the matched installed device. */
  target_hardware_id: string | null;
  support_url: string | null;
}

export interface SystemDeviceGroup {
  class: SystemDeviceClass;
  label: string;
  updates: SystemDriverUpdate[];
}

export type SystemInstallStage = "downloading" | "installing" | "completed" | "failed";

export interface SystemDriverProgress {
  stage: SystemInstallStage;
  message: string;
  fraction: number | null;
}

export interface SystemDriverOutcome {
  success: boolean;
  reboot_required: boolean;
  result_code: number;
  message: string;
}

export const SYSTEM_DRIVER_INSTALL_EVENT = "system_driver_install_progress";

/** Installed-device context so the install snapshots the current driver before applying. */
export interface DriverInstallContext {
  infName: string | null;
  hardwareId: string | null;
  deviceClass: string | null;
  provider: string | null;
  currentVersion: string | null;
}

/** One DriverStore version (current or superseded) of a driver package. */
export interface DriverStoreVersion {
  publishedName: string;
  version: string;
  date: string | null;
  provider: string;
  current: boolean;
}

/** Build the snapshot context for a System & Components update from its matched device. */
export function driverInstallContext(
  update: SystemDriverUpdate,
  deviceClass: string,
): DriverInstallContext {
  return {
    infName: update.target_inf,
    hardwareId: update.target_hardware_id,
    deviceClass,
    provider: update.provider,
    currentVersion: update.current_version,
  };
}

export async function scanSystemDrivers(): Promise<SystemDeviceGroup[]> {
  return invoke("scan_system_drivers");
}

export async function installSystemDriver(
  updateId: string,
  context?: DriverInstallContext,
): Promise<SystemDriverOutcome> {
  return invoke("install_system_driver", { updateId, context: context ?? null });
}

/** Roll a System & Components driver back to a previously-snapshotted version. */
export async function restoreSystemDriver(backupId: string): Promise<SystemDriverOutcome> {
  return invoke("restore_system_driver", { backupId });
}

/** DriverStore versions (current + superseded) of a driver package, newest-first. */
export async function systemDriverVersions(infName: string): Promise<DriverStoreVersion[]> {
  return invoke("system_driver_versions", { infName });
}

export type DlssPreset =
  | "default"
  | "a"
  | "b"
  | "c"
  | "d"
  | "e"
  | "f"
  | "g"
  | "h"
  | "i"
  | "j"
  | "k"
  | "l"
  | "m"
  | "n"
  | "o"
  | "recommended";

export type FrameGenMode = "app_controlled" | "fixed" | "dynamic";
export type FrameGenCount = "app_controlled" | "x2" | "x3" | "x4";

export interface DlssOverrideConfig {
  enable_sr_dll_override: boolean;
  sr_preset: DlssPreset | null;
  enable_fg_dll_override: boolean;
  fg_preset: DlssPreset | null;
  fg_mode: FrameGenMode | null;
  fg_fixed_count: FrameGenCount | null;
  fg_dynamic_target_fps: number | null;
}

export type OverrideScope = { scope: "global" } | { scope: "per_game"; executable_path: string };

export type DlssOverrideSource = "per_game" | "global" | "none";

export interface DlssOverrideReadback {
  config: DlssOverrideConfig;
  source: DlssOverrideSource;
  active_count: number;
}

export async function dlssOverridesSupported(): Promise<boolean> {
  return invoke("dlss_overrides_supported");
}

export interface DlssApplyOutcome {
  needs_elevation: boolean;
  denied_settings: number[];
}

export async function applyDlssOverride(
  scope: OverrideScope,
  config: DlssOverrideConfig,
): Promise<DlssApplyOutcome> {
  return invoke("apply_dlss_override", { scope, config });
}

export async function resetDlssOverride(scope: OverrideScope): Promise<void> {
  return invoke("reset_dlss_override", { scope });
}

export async function readDlssOverrideConfig(scope: OverrideScope): Promise<DlssOverrideReadback> {
  return invoke("read_dlss_override_config", { scope });
}

export async function findGameExecutable(installDir: string): Promise<string | null> {
  return invoke("find_game_executable", { installDir });
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

export async function applyUpdateBatch(request: ApplyBatchRequest): Promise<ApplyBatchResult> {
  return invoke("apply_update_batch", { request });
}

export async function applyStreamlineSet(items: ApplyRequest[]): Promise<StreamlineSetResult> {
  return invoke("apply_streamline_set", { items });
}

export async function cancelApply(applyId: string): Promise<boolean> {
  return invoke("cancel_apply", { applyId });
}

export async function cancelAllApplies(): Promise<number> {
  return invoke("cancel_all_applies");
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

export async function openUrl(url: string): Promise<void> {
  const { open } = await import("@tauri-apps/plugin-shell");
  await open(url);
}

export async function revealPath(path: string): Promise<void> {
  return invoke("reveal_path", { path });
}

export interface LogPaths {
  logs_dir: string;
  current_log: string | null;
  file_count: number;
}

export interface IssueReport {
  url: string;
  body: string;
}

export async function getLogPaths(): Promise<LogPaths> {
  return invoke("get_log_paths");
}

export async function readRecentLogs(maxLines?: number): Promise<string> {
  return invoke("read_recent_logs", { maxLines });
}

export async function buildIssueReport(context?: string): Promise<IssueReport> {
  return invoke("build_issue_report", { context });
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

/** Set the tray tooltip/badge to the count of games with pending updates.
 *  0 reverts the tray to its idle tooltip. */
export async function traySetPending(count: number): Promise<void> {
  return invoke("tray_set_pending", { count });
}
