import type {
  LibraryViewMode,
  LibraryDensity,
  LibrarySort,
  BackupsGroupBy,
  SettingsTab,
} from "./api";

export const LIBRARY_VIEW_MODES: readonly LibraryViewMode[] = ["grid", "list"];
export const LIBRARY_DENSITIES: readonly LibraryDensity[] = ["compact", "comfy"];
export const LIBRARY_SORTS: readonly LibrarySort[] = [
  "default",
  "outdated_first",
  "a_z",
  "z_a",
  "launcher",
];
export const BACKUPS_GROUP_BYS: readonly BackupsGroupBy[] = ["game", "date"];
export const SETTINGS_TABS: readonly SettingsTab[] = [
  "general",
  "updates",
  "detection",
  "art",
  "advanced",
];

export const LIBRARY_VIEW_MODE_DEFAULT: LibraryViewMode = "grid";
export const LIBRARY_DENSITY_DEFAULT: LibraryDensity = "comfy";
export const LIBRARY_SORT_DEFAULT: LibrarySort = "default";
export const BACKUPS_GROUP_BY_DEFAULT: BackupsGroupBy = "game";
export const SETTINGS_TAB_DEFAULT: SettingsTab = "general";

export const SIDEBAR_RAIL_BREAKPOINT_PX = 1300;
export const CATALOG_BENTO_BREAKPOINT_PX = 1100;
export const COMMAND_PALETTE_RECENT_MAX = 5;
export const COMMAND_PALETTE_MAX_WIDTH_PX = 560;
export const COMMAND_PALETTE_MAX_HEIGHT_PX = 480;
export const COMMAND_PALETTE_OPEN_BUDGET_MS = 120;
export const NOTIFICATIONS_FIFO_CAP = 200;
export const APPLY_HERO_DRAWER_THRESHOLD = 1;
export const NO_DLLS_STAGGER_ITEM_LIMIT = 13;

export const ANIMATION_DURATIONS_MS = {
  instant: 80,
  fast: 140,
  normal: 220,
  slow: 360,
  stagger: 12,
} as const;

export type LibrarySortLabel = { id: LibrarySort; label: string; hint?: string };
export const LIBRARY_SORT_LABELS: readonly LibrarySortLabel[] = [
  { id: "default", label: "Recommended", hint: "Outdated games first, then A → Z" },
  { id: "outdated_first", label: "Outdated first", hint: "Games with pending updates on top" },
  { id: "a_z", label: "Name A → Z" },
  { id: "z_a", label: "Name Z → A" },
  { id: "launcher", label: "Launcher", hint: "Grouped by launcher, then name" },
];

export type ShortcutScope = "global" | "library" | "drawer" | "modal" | "palette";

export interface Shortcut {
  scope: ShortcutScope;
  keys: readonly string[];
  description: string;
}

export const SHORTCUTS: readonly Shortcut[] = [
  { scope: "global", keys: ["mod", "k"], description: "Open command palette" },
  { scope: "global", keys: ["?"], description: "Show keyboard shortcuts" },
  { scope: "global", keys: ["/"], description: "Focus search" },
  { scope: "global", keys: ["g", "l"], description: "Go to Library" },
  { scope: "global", keys: ["g", "c"], description: "Go to Catalog" },
  { scope: "global", keys: ["g", "b"], description: "Go to Backups" },
  { scope: "global", keys: ["g", "s"], description: "Go to Settings" },
  { scope: "global", keys: ["g", "a"], description: "Go to About" },
  { scope: "global", keys: ["esc"], description: "Close palette / modal / drawer" },
  { scope: "library", keys: ["a"], description: "Apply all outdated updates" },
  { scope: "library", keys: ["r"], description: "Rescan installed games" },
  { scope: "library", keys: ["v"], description: "Toggle Grid / List view" },
  { scope: "library", keys: ["d"], description: "Toggle Compact / Comfy density" },
  { scope: "drawer", keys: ["arrowdown"], description: "Next feature row" },
  { scope: "drawer", keys: ["arrowup"], description: "Previous feature row" },
  { scope: "drawer", keys: ["space"], description: "Toggle feature selection" },
  { scope: "drawer", keys: ["enter"], description: "Open version picker" },
  { scope: "modal", keys: ["esc"], description: "Cancel running apply" },
  { scope: "palette", keys: ["tab"], description: "Cycle category filter" },
  { scope: "palette", keys: ["enter"], description: "Run selected command" },
];

export type CommandCategory = "navigate" | "action" | "settings";

export interface PaletteCommand {
  id: string;
  title: string;
  aliases: readonly string[];
  category: CommandCategory;
  /** Lucide icon key (kebab-case file name) resolved to a component in the palette. */
  icon: string;
  /** Key sequence shown as a chip, e.g. ["g","l"]. Display-only — the global handlers own the bindings. */
  shortcut?: readonly string[];
  hint?: string;
}

export const COMMANDS: readonly PaletteCommand[] = [
  { id: "nav.library", title: "Go to Library", aliases: ["library", "games"], category: "navigate", icon: "layout-grid", shortcut: ["g", "l"], hint: "Your installed games and pending updates" },
  { id: "nav.catalog", title: "Go to Catalog", aliases: ["catalog", "versions", "manifest"], category: "navigate", icon: "layers", shortcut: ["g", "c"], hint: "Every DLL version in the manifest" },
  { id: "nav.backups", title: "Go to Backups", aliases: ["backups", "snapshots", "restore"], category: "navigate", icon: "archive", shortcut: ["g", "b"], hint: "Saved snapshots you can restore" },
  { id: "nav.settings", title: "Go to Settings", aliases: ["settings", "prefs", "preferences", "options"], category: "navigate", icon: "settings", shortcut: ["g", "s"] },
  { id: "nav.about", title: "Go to About", aliases: ["about", "info", "version", "system"], category: "navigate", icon: "info", shortcut: ["g", "a"] },

  { id: "action.apply_all_outdated", title: "Apply all outdated updates", aliases: ["apply", "update all", "run updates"], category: "action", icon: "download-cloud", shortcut: ["a"] },
  { id: "action.rescan", title: "Rescan installed games", aliases: ["rescan", "refresh games"], category: "action", icon: "refresh-cw", shortcut: ["r"] },
  { id: "action.refresh_manifest", title: "Refresh DLL manifest", aliases: ["refresh manifest", "catalog refresh"], category: "action", icon: "rotate-cw" },
  { id: "action.check_updates", title: "Check for app updates", aliases: ["check updates", "upgrade"], category: "action", icon: "arrow-up-circle" },
  { id: "action.restore_recent", title: "Restore most recent backup", aliases: ["restore", "undo", "revert"], category: "action", icon: "undo-2" },
  { id: "action.open_data_folder", title: "Open DLSSync data folder", aliases: ["open data", "reveal data folder"], category: "action", icon: "folder" },
  { id: "action.open_backups_folder", title: "Open backups folder", aliases: ["open backups", "reveal backups"], category: "action", icon: "folder-archive" },
  { id: "action.open_logs_folder", title: "Open logs folder", aliases: ["open logs", "reveal logs"], category: "action", icon: "scroll-text" },
  { id: "action.toggle_theme", title: "Toggle dark / light theme", aliases: ["toggle theme", "dark mode", "light mode"], category: "action", icon: "sun-moon" },
  { id: "action.toggle_view_mode", title: "Toggle Library view (Grid / List)", aliases: ["toggle view", "grid", "list"], category: "action", icon: "layout-list", shortcut: ["v"] },
  { id: "action.toggle_density", title: "Toggle Library density (Compact / Comfy)", aliases: ["toggle density", "compact", "comfy"], category: "action", icon: "rows-3", shortcut: ["d"] },

  { id: "settings.general", title: "Settings · General", aliases: ["general", "startup"], category: "settings", icon: "sliders-horizontal" },
  { id: "settings.updates", title: "Settings · Update preferences", aliases: ["update preferences", "vendor toggles"], category: "settings", icon: "download-cloud" },
  { id: "settings.detection", title: "Settings · Detection", aliases: ["detection", "scanners", "launchers"], category: "settings", icon: "radar" },
  { id: "settings.art", title: "Settings · Art", aliases: ["art", "covers", "steamgriddb"], category: "settings", icon: "image" },
  { id: "settings.advanced", title: "Settings · Advanced", aliases: ["advanced", "retries", "concurrency", "allow unsigned"], category: "settings", icon: "wrench" },
];

export const COMMAND_CATEGORY_LABELS: Record<CommandCategory | "all", string> = {
  all: "All",
  navigate: "Navigate",
  action: "Action",
  settings: "Settings",
};

export type CommandMatch = { command: PaletteCommand; score: number };

export function matchCommands(query: string, commands: readonly PaletteCommand[]): CommandMatch[] {
  const q = query.toLowerCase().trim();
  if (!q) {
    return commands.map((command) => ({ command, score: 0 }));
  }
  const out: CommandMatch[] = [];
  for (const command of commands) {
    const haystacks: string[] = [command.title.toLowerCase(), ...command.aliases.map((a) => a.toLowerCase())];
    let best: number | null = null;
    for (const h of haystacks) {
      const s = subsequenceScore(q, h);
      if (s !== null && (best === null || s < best)) best = s;
    }
    if (best !== null) out.push({ command, score: best });
  }
  out.sort((a, b) => a.score - b.score);
  return out;
}

/** Indices in `text` that the query matched, for fuzzy-character highlighting.
 *  Prefers a contiguous substring span; falls back to subsequence positions;
 *  returns [] when the query is not a subsequence of `text` (e.g. it matched an alias). */
export function matchedIndices(query: string, text: string): number[] {
  const q = query.toLowerCase().trim();
  if (!q) return [];
  const t = text.toLowerCase();
  const span = t.indexOf(q);
  if (span >= 0) {
    return Array.from({ length: q.length }, (_, k) => span + k);
  }
  const out: number[] = [];
  let i = 0;
  for (let pos = 0; pos < t.length && i < q.length; pos++) {
    if (t[pos] === q[i]) {
      out.push(pos);
      i += 1;
    }
  }
  return i === q.length ? out : [];
}

export interface HighlightSegment {
  text: string;
  hit: boolean;
}

/** Split `text` into consecutive hit / non-hit segments from matched indices. */
export function highlightSegments(text: string, indices: readonly number[]): HighlightSegment[] {
  if (indices.length === 0) return [{ text, hit: false }];
  const flags = new Set(indices);
  const segments: HighlightSegment[] = [];
  let buffer = "";
  let bufferHit = flags.has(0);
  for (let i = 0; i < text.length; i++) {
    const hit = flags.has(i);
    if (hit !== bufferHit) {
      if (buffer) segments.push({ text: buffer, hit: bufferHit });
      buffer = "";
      bufferHit = hit;
    }
    buffer += text[i];
  }
  if (buffer) segments.push({ text: buffer, hit: bufferHit });
  return segments;
}

function subsequenceScore(needle: string, haystack: string): number | null {
  if (haystack.includes(needle)) return haystack.length - needle.length;
  let i = 0;
  for (const ch of haystack) {
    if (i < needle.length && ch === needle[i]) i++;
    if (i === needle.length) break;
  }
  if (i !== needle.length) return null;
  return haystack.length - needle.length + 50;
}

export const EXTERNAL_URLS = {
  anticheatFaq: "https://www.pcgamingwiki.com/wiki/Glossary:Anti-cheat",
  homepage: "https://github.com/xt0n1-t3ch/DLSSync",
  releases: "https://github.com/xt0n1-t3ch/DLSSync/releases",
  releasesLatest: "https://github.com/xt0n1-t3ch/DLSSync/releases/latest",
  nexusMod: "https://www.nexusmods.com/site/mods/1922",
  sponsor: "https://github.com/sponsors/xt0n1-t3ch",
  kofi: "https://ko-fi.com/xt0n1",
} as const;

export function githubReleaseTagUrl(version: string): string {
  const tag = version.trim().replace(/^v/i, "");
  return `${EXTERNAL_URLS.homepage}/releases/tag/v${tag}`;
}

/** Why a driver install asks for Administrator — shown wherever a per-machine
 *  install can trigger a UAC prompt (System & Components, GPU drivers). */
export const ADMIN_ELEVATION_NOTE =
  "Windows installs per-machine drivers only with Administrator rights, so Update shows a UAC prompt. DLSSync stays unelevated and runs a signed helper with your approval — it snapshots the current driver and sets a System Restore point first, so you can roll back.";

export const VENDOR_TOKEN_BY_FAMILY: Record<string, string> = {
  dlss_sr: "var(--vendor-nvidia)",
  dlss_fg: "var(--vendor-nvidia)",
  dlss_rr: "var(--vendor-nvidia)",
  reflex: "var(--vendor-nvidia)",
  streamline: "var(--vendor-nvidia)",
  streamline_common: "var(--vendor-nvidia)",
  streamline_pcl: "var(--vendor-nvidia)",
  streamline_nis: "var(--vendor-nvidia)",
  streamline_direct_sr: "var(--vendor-microsoft)",
  xess_sr: "var(--vendor-intel)",
  xess_sr_dx11: "var(--vendor-intel)",
  xess_fg: "var(--vendor-intel)",
  xell: "var(--vendor-intel)",
  fsr_upscaler: "var(--vendor-amd)",
  fsr_upscaler_vk: "var(--vendor-amd)",
  fsr_fg: "var(--vendor-amd)",
  fsr_loader: "var(--vendor-amd)",
  direct_storage: "var(--vendor-microsoft)",
};

export type VendorKey = "nvidia" | "amd" | "intel" | "microsoft";

export const VENDOR_LABELS: Record<VendorKey, string> = {
  nvidia: "NVIDIA",
  amd: "AMD",
  intel: "Intel",
  microsoft: "Microsoft",
};

export function vendorForFamily(family: string): VendorKey | null {
  if (family.startsWith("dlss") || family === "reflex" || family === "streamline" || family.startsWith("streamline_")) {
    if (family === "streamline_direct_sr") return "microsoft";
    return "nvidia";
  }
  if (family.startsWith("xess") || family === "xell") return "intel";
  if (family.startsWith("fsr")) return "amd";
  if (family === "direct_storage") return "microsoft";
  return null;
}

export function isModifierComboMatch(event: KeyboardEvent, keys: readonly string[]): boolean {
  if (keys.length === 0) return false;
  const expected = keys[keys.length - 1].toLowerCase();
  const actual = event.key.toLowerCase();
  if (actual !== expected && actual !== keyAliasFor(expected)) return false;
  const wantsMod = keys.includes("mod");
  const hasMod = event.metaKey || event.ctrlKey;
  if (wantsMod !== hasMod) return false;
  const wantsShift = keys.includes("shift");
  if (wantsShift !== event.shiftKey) return false;
  return true;
}

function keyAliasFor(key: string): string {
  switch (key) {
    case "esc":
      return "escape";
    case "space":
      return " ";
    default:
      return key;
  }
}

export function pushRecentCommand(recent: readonly string[], id: string): string[] {
  const next = [id, ...recent.filter((r) => r !== id)];
  return next.slice(0, COMMAND_PALETTE_RECENT_MAX);
}
