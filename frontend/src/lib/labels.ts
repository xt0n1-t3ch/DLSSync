import type { DllFamily, DllRecord, LauncherKind } from "./api";

export const VENDOR_LABELS: Record<string, string> = {
  nvidia: "NVIDIA",
  amd: "AMD",
  intel: "Intel",
  microsoft: "Microsoft",
  other: "GPU",
};

export const VENDOR_ACCENTS: Record<string, string> = {
  nvidia: "#76b900",
  amd: "#ed1c24",
  intel: "#0071c5",
  microsoft: "#00a4ef",
};

export const DEFAULT_VENDOR_ACCENT = "#94a3b8";

export function vendorAccent(key: string): string {
  return VENDOR_ACCENTS[key] ?? DEFAULT_VENDOR_ACCENT;
}

export const VENDOR_ACCENT_VARS: Record<string, string> = {
  nvidia: "var(--vendor-nvidia)",
  amd: "var(--vendor-amd)",
  intel: "var(--vendor-intel)",
  microsoft: "var(--vendor-microsoft)",
};

export const DEFAULT_VENDOR_ACCENT_VAR = "var(--neutral)";

export function vendorAccentVar(key: string): string {
  return VENDOR_ACCENT_VARS[key] ?? DEFAULT_VENDOR_ACCENT_VAR;
}

export interface VendorPortal {
  label: string;
  url: string;
}

export const VENDOR_PORTALS: Record<string, VendorPortal> = {
  nvidia: { label: "NVIDIA Developer DLSS", url: "https://developer.nvidia.com/rtx/dlss" },
  intel: { label: "Intel XeSS Releases", url: "https://github.com/intel/xess/releases" },
  amd: { label: "AMD GPUOpen FSR SDK", url: "https://gpuopen.com/amd-fidelityfx-sdk/" },
  microsoft: { label: "DirectStorage NuGet", url: "https://www.nuget.org/packages/Microsoft.Direct3D.DirectStorage" },
};

export function vendorPortal(key: string): VendorPortal | null {
  return VENDOR_PORTALS[key] ?? null;
}

export function launcherAccent(key: LauncherKind | string): string {
  return LAUNCHER_ACCENTS[key as LauncherKind] ?? DEFAULT_VENDOR_ACCENT;
}

export const FAMILY_LABELS: Record<string, string> = {
  dlss_sr: "DLSS Super Resolution",
  dlss_fg: "DLSS Frame Generation",
  dlss_rr: "DLSS Ray Reconstruction",
  sl_dlss_sr: "DLSS Super Resolution (Streamline plug-in)",
  sl_dlss_fg: "DLSS Frame Generation (Streamline plug-in)",
  sl_dlss_rr: "DLSS Ray Reconstruction (Streamline plug-in)",
  streamline: "Streamline SDK",
  streamline_common: "Streamline Common",
  streamline_pcl: "Streamline PCL",
  streamline_nis: "Streamline NIS",
  streamline_direct_sr: "Streamline DirectSR",
  reflex: "Reflex",
  xess_sr: "XeSS Super Resolution",
  xess_sr_dx11: "XeSS DX11",
  xess_fg: "XeSS Frame Generation",
  xell: "XeLL",
  fsr_upscaler: "FSR Upscaler",
  fsr_upscaler_vk: "FSR Vulkan",
  fsr_fg: "FSR Frame Generation",
  fsr_loader: "FSR Loader",
  fsr_denoiser: "FSR Denoiser",
  direct_storage: "DirectStorage",
  direct_storage_core: "DirectStorage Core",
};

export const FAMILY_SHORT: Record<string, string> = {
  dlss_sr: "DLSS",
  dlss_fg: "DLSS FG",
  dlss_rr: "DLSS RR",
  sl_dlss_sr: "SL DLSS",
  sl_dlss_fg: "SL DLSS FG",
  sl_dlss_rr: "SL DLSS RR",
  streamline: "Streamline",
  streamline_common: "SL Common",
  streamline_pcl: "SL PCL",
  streamline_nis: "SL NIS",
  streamline_direct_sr: "DirectSR",
  reflex: "Reflex",
  xess_sr: "XeSS",
  xess_sr_dx11: "XeSS DX11",
  xess_fg: "XeSS FG",
  xell: "XeLL",
  fsr_upscaler: "FSR",
  fsr_upscaler_vk: "FSR VK",
  fsr_fg: "FSR FG",
  fsr_loader: "FSR Loader",
  fsr_denoiser: "FSR Denoiser",
  direct_storage: "DirectStorage",
  direct_storage_core: "DS Core",
};

export const FAMILY_TO_VENDOR: Record<string, string> = {
  dlss_sr: "nvidia",
  dlss_fg: "nvidia",
  dlss_rr: "nvidia",
  sl_dlss_sr: "nvidia",
  sl_dlss_fg: "nvidia",
  sl_dlss_rr: "nvidia",
  streamline: "nvidia",
  streamline_common: "nvidia",
  streamline_pcl: "nvidia",
  streamline_nis: "nvidia",
  streamline_direct_sr: "nvidia",
  reflex: "nvidia",
  xess_sr: "intel",
  xess_sr_dx11: "intel",
  xess_fg: "intel",
  xell: "intel",
  fsr_upscaler: "amd",
  fsr_upscaler_vk: "amd",
  fsr_fg: "amd",
  fsr_loader: "amd",
  fsr_denoiser: "amd",
  direct_storage: "microsoft",
  direct_storage_core: "microsoft",
};

export const FAMILY_TO_CATALOG_KEY: Record<string, string> = {
  dlss_sr: "dlss_sr",
  dlss_fg: "dlss_fg",
  dlss_rr: "dlss_rr",
  sl_dlss_sr: "sl_dlss_sr",
  sl_dlss_fg: "sl_dlss_fg",
  sl_dlss_rr: "sl_dlss_rr",
  streamline: "streamline",
  streamline_common: "streamline",
  streamline_pcl: "streamline",
  streamline_nis: "streamline",
  streamline_direct_sr: "streamline",
  reflex: "reflex",
  xess_sr: "xess_sr",
  xess_sr_dx11: "xess_sr",
  xess_fg: "xess_fg",
  xell: "xell",
  fsr_upscaler: "fsr_upscaler",
  fsr_upscaler_vk: "fsr_upscaler",
  fsr_loader: "fsr_upscaler",
  fsr_fg: "fsr_fg",
  fsr_denoiser: "fsr_denoiser",
  direct_storage: "direct_storage",
  direct_storage_core: "direct_storage_core",
};

export const LAUNCHER_LABELS: Record<LauncherKind, string> = {
  steam: "Steam",
  epic: "Epic Games",
  gog: "GOG",
  ubisoft: "Ubisoft",
  ea_desktop: "EA Desktop",
  xbox: "Xbox",
  battlenet: "Battle.net",
  manual: "Manual",
};

export const LAUNCHER_ACCENTS: Record<LauncherKind, string> = {
  steam: "#66c0f4",
  epic: "#f5f5f5",
  gog: "#a4a4f4",
  ubisoft: "#00aaff",
  ea_desktop: "#ff5050",
  xbox: "#107c10",
  battlenet: "#148eff",
  manual: "#22d3ee",
};

export type UpdateStatus = "outdated" | "up_to_date" | "no_dlls" | "unknown" | "scanning" | "scan_failed";

export const STATUS_LABELS: Record<UpdateStatus, string> = {
  outdated: "Update available",
  up_to_date: "Up to date",
  no_dlls: "No DLLs",
  unknown: "Unknown",
  scanning: "Scanning",
  scan_failed: "Scan failed",
};

export function vendorLabel(key: string): string {
  return VENDOR_LABELS[key] ?? key;
}

export function familyLabel(key: string): string {
  return FAMILY_LABELS[key] ?? key;
}

export function familyShort(key: string): string {
  return FAMILY_SHORT[key] ?? key;
}

export function launcherLabel(key: LauncherKind): string {
  return LAUNCHER_LABELS[key] ?? key;
}

export function familyVendor(key: DllFamily): string {
  return FAMILY_TO_VENDOR[key] ?? "nvidia";
}

export function familyCatalogKey(key: DllFamily): string {
  return FAMILY_TO_CATALOG_KEY[key] ?? key;
}

export type FamilyGroup = "dlss" | "fsr" | "xess" | "advanced";

export const GROUP_LABELS: Record<FamilyGroup, string> = {
  dlss: "DLSS",
  fsr: "AMD FSR",
  xess: "Intel XeSS",
  advanced: "Other technologies",
};

export const GROUP_SUB: Record<FamilyGroup, string> = {
  dlss: "NVIDIA AI upscaling, frame generation and ray reconstruction",
  fsr: "AMD upscaling and frame generation",
  xess: "Intel AI upscaling and frame generation",
  advanced: "NVIDIA Reflex, Streamline plug-ins, Microsoft DirectStorage",
};

export const GROUP_ACCENT: Record<FamilyGroup, string> = {
  dlss: "#76b900",
  fsr: "#ed1c24",
  xess: "#0071c5",
  advanced: "#94a3b8",
};

export const GROUP_ACCENT_VAR: Record<FamilyGroup, string> = {
  dlss: "var(--vendor-nvidia)",
  fsr: "var(--vendor-amd)",
  xess: "var(--vendor-intel)",
  advanced: "var(--neutral)",
};

export const FAMILY_TO_GROUP: Record<string, FamilyGroup> = {
  dlss_sr: "dlss",
  dlss_fg: "dlss",
  dlss_rr: "dlss",
  sl_dlss_sr: "dlss",
  sl_dlss_fg: "dlss",
  sl_dlss_rr: "dlss",
  fsr_upscaler: "fsr",
  fsr_upscaler_vk: "fsr",
  fsr_fg: "fsr",
  fsr_loader: "fsr",
  fsr_denoiser: "fsr",
  xess_sr: "xess",
  xess_sr_dx11: "xess",
  xess_fg: "xess",
  xell: "xess",
  streamline: "advanced",
  streamline_common: "advanced",
  streamline_pcl: "advanced",
  streamline_nis: "advanced",
  streamline_direct_sr: "advanced",
  reflex: "advanced",
  direct_storage: "advanced",
};

export function familyGroup(key: DllFamily | string): FamilyGroup {
  return FAMILY_TO_GROUP[key] ?? "advanced";
}

export const GROUP_ORDER: FamilyGroup[] = ["dlss", "fsr", "xess", "advanced"];

export type FeatureId =
  | "dlss_sr"
  | "dlss_fg"
  | "dlss_rr"
  | "fsr_upscaler"
  | "xess_sr"
  | "xess_fg";

export type FeatureSlot = FeatureId | "advanced";

export interface FeatureDef {
  id: FeatureId;
  title: string;
  short: string;
  blurb: string;
  iconId: string;
  group: FamilyGroup;
  vendor: string;
  catalogKey: string;
}

export const FEATURE_DEFS: Record<FeatureId, FeatureDef> = {
  dlss_sr: {
    id: "dlss_sr",
    title: "DLSS Super Resolution",
    short: "DLSS",
    blurb: "Sharper image at higher FPS — NVIDIA AI upscaling",
    iconId: "dlss",
    group: "dlss",
    vendor: "nvidia",
    catalogKey: "dlss_sr",
  },
  dlss_fg: {
    id: "dlss_fg",
    title: "DLSS Frame Generation",
    short: "Frame Gen",
    blurb: "Extra interpolated frames for smoother motion",
    iconId: "frame_gen",
    group: "dlss",
    vendor: "nvidia",
    catalogKey: "dlss_fg",
  },
  dlss_rr: {
    id: "dlss_rr",
    title: "DLSS Ray Reconstruction",
    short: "Ray Recon",
    blurb: "Cleaner ray-traced reflections, shadows and global illumination",
    iconId: "ray_recon",
    group: "dlss",
    vendor: "nvidia",
    catalogKey: "dlss_rr",
  },
  fsr_upscaler: {
    id: "fsr_upscaler",
    title: "FSR Upscaling",
    short: "FSR",
    blurb: "AMD spatial and temporal upscaling — works on any GPU",
    iconId: "fsr",
    group: "fsr",
    vendor: "amd",
    catalogKey: "fsr_upscaler",
  },
  xess_sr: {
    id: "xess_sr",
    title: "Intel XeSS",
    short: "XeSS",
    blurb: "Intel AI upscaling — best on Arc, works elsewhere",
    iconId: "xess",
    group: "xess",
    vendor: "intel",
    catalogKey: "xess_sr",
  },
  xess_fg: {
    id: "xess_fg",
    title: "XeSS Frame Generation",
    short: "XeSS FG",
    blurb: "Intel frame interpolation for higher FPS",
    iconId: "xess_fg",
    group: "xess",
    vendor: "intel",
    catalogKey: "xess_fg",
  },
};

export const FEATURE_ORDER: FeatureId[] = [
  "dlss_sr",
  "dlss_fg",
  "dlss_rr",
  "fsr_upscaler",
  "xess_sr",
  "xess_fg",
];

const STREAMLINE_FILE_TO_FEATURE: { suffix: string; feature: FeatureSlot }[] = [
  { suffix: "sl.dlss_g.dll", feature: "dlss_fg" },
  { suffix: "sl.dlss_d.dll", feature: "dlss_rr" },
  { suffix: "sl.dlss.dll", feature: "dlss_sr" },
];

const FAMILY_TO_FEATURE: Record<string, FeatureSlot> = {
  dlss_sr: "dlss_sr",
  dlss_fg: "dlss_fg",
  dlss_rr: "dlss_rr",
  sl_dlss_sr: "dlss_sr",
  sl_dlss_fg: "dlss_fg",
  sl_dlss_rr: "dlss_rr",
  fsr_upscaler: "fsr_upscaler",
  fsr_upscaler_vk: "fsr_upscaler",
  fsr_loader: "fsr_upscaler",
  fsr_fg: "advanced",
  xess_sr: "xess_sr",
  xess_sr_dx11: "xess_sr",
  xell: "xess_sr",
  xess_fg: "xess_fg",
  streamline: "advanced",
  streamline_common: "advanced",
  streamline_pcl: "advanced",
  streamline_nis: "advanced",
  streamline_direct_sr: "advanced",
  reflex: "advanced",
  direct_storage: "advanced",
};

export function recordFeature(record: DllRecord): FeatureSlot {
  if (record.family === "streamline") {
    const fname = filenameFromPath(record.path).toLowerCase();
    for (const m of STREAMLINE_FILE_TO_FEATURE) {
      if (fname.endsWith(m.suffix)) return m.feature;
    }
    return "advanced";
  }
  return FAMILY_TO_FEATURE[record.family] ?? "advanced";
}

export function featureFromFamily(family: string): FeatureSlot {
  return FAMILY_TO_FEATURE[family] ?? "advanced";
}

export function featureTitle(id: FeatureSlot): string {
  if (id === "advanced") return GROUP_LABELS.advanced;
  return FEATURE_DEFS[id].title;
}

export function featureShort(id: FeatureSlot): string {
  if (id === "advanced") return "Other";
  return FEATURE_DEFS[id].short;
}

export function featureBlurb(id: FeatureSlot): string {
  if (id === "advanced") return GROUP_SUB.advanced;
  return FEATURE_DEFS[id].blurb;
}

export function featureGroup(id: FeatureSlot): FamilyGroup {
  if (id === "advanced") return "advanced";
  return FEATURE_DEFS[id].group;
}

export function featureVendor(id: FeatureSlot): string {
  if (id === "advanced") return "microsoft";
  return FEATURE_DEFS[id].vendor;
}

export function featureIconId(id: FeatureSlot): string {
  if (id === "advanced") return "advanced";
  return FEATURE_DEFS[id].iconId;
}

export function filenameFromPath(p: string): string {
  const m = p.match(/[\/\\]([^\/\\]+)$/);
  return m ? m[1] : p;
}

export const VIEW_TITLES: Record<string, string> = {
  library: "Library",
  catalog: "Catalog",
  backups: "Backups",
  settings: "Settings",
  about: "About",
};

export function viewTitle(slug: string): string {
  return VIEW_TITLES[slug] ?? "";
}
