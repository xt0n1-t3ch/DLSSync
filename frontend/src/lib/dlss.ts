import type { DlssOverrideConfig, DlssPreset, FrameGenCount, FrameGenMode } from "./api";

export interface Option<T> {
  value: T;
  label: string;
  description: string;
  sourceUrl: string;
}

const SRC = {
  streamline: "https://github.com/NVIDIAGameWorks/Streamline/blob/main/include/sl_dlss.h",
  dlss45:
    "https://www.nvidia.com/en-us/geforce/news/dlss-4-5-dynamic-multi-frame-gen-6x-2nd-gen-transformer-super-res/",
  dlssOverview: "https://developer.nvidia.com/rtx/dlss",
  presetTable: "https://en.wikipedia.org/wiki/Nvidia_DLSS",
  dynamicMfg:
    "https://www.nvidia.com/en-us/geforce/news/dlss-4-5-dynamic-multi-frame-generation-6x-mode-released/",
} as const;

export const SR_PRESET_OPTIONS: Option<DlssPreset>[] = [
  {
    value: "recommended",
    label: "Recommended (latest)",
    description:
      "Lets NVIDIA pick the best model per quality mode — Preset M for Performance, L for Ultra Performance, K for the rest. Best default for most users.",
    sourceUrl: SRC.dlss45,
  },
  {
    value: "default",
    label: "Use app default",
    description: "Leave the game's own DLSS model untouched — no preset is forced.",
    sourceUrl: SRC.dlssOverview,
  },
  {
    value: "k",
    label: "Preset K — Transformer (latest)",
    description:
      "DLSS 4 transformer model. Best image quality — sharper, more stable, less ghosting — at a higher GPU cost. Default for DLAA / Quality / Balanced.",
    sourceUrl: SRC.streamline,
  },
  {
    value: "j",
    label: "Preset J — Transformer",
    description:
      "DLSS 4 transformer, close to K with slightly less ghosting but a touch more flicker. K is generally preferred over J.",
    sourceUrl: SRC.streamline,
  },
  {
    value: "l",
    label: "Preset L — Transformer (Ultra Perf)",
    description:
      "DLSS 4.5 second-gen transformer tuned for Ultra Performance / 4K — sharpest and most stable, highest cost. RTX 20/30 lack FP8 so it is heavier there.",
    sourceUrl: SRC.dlss45,
  },
  {
    value: "m",
    label: "Preset M — Transformer (Perf)",
    description:
      "DLSS 4.5 second-gen transformer tuned for Performance mode — near-L quality at roughly J/K speed.",
    sourceUrl: SRC.dlss45,
  },
  {
    value: "e",
    label: "Preset E — CNN (legacy)",
    description:
      "Legacy convolutional model. Prefer a transformer preset (K) on RTX unless a specific game misbehaves with it.",
    sourceUrl: SRC.presetTable,
  },
  {
    value: "f",
    label: "Preset F — CNN (legacy)",
    description: "Legacy CNN tuned for Ultra Performance / DLAA at 4K and above.",
    sourceUrl: SRC.presetTable,
  },
  {
    value: "c",
    label: "Preset C — CNN (legacy)",
    description:
      "Legacy CNN variant for fast-paced games — less ghosting at the cost of temporal stability.",
    sourceUrl: SRC.presetTable,
  },
  {
    value: "d",
    label: "Preset D — CNN (legacy)",
    description:
      "Legacy CNN variant for slower-paced games — more temporally stable but more ghosting.",
    sourceUrl: SRC.presetTable,
  },
];

export const FG_MODE_OPTIONS: Option<FrameGenMode>[] = [
  {
    value: "app_controlled",
    label: "Use app setting",
    description: "Keep the in-game Frame Generation setting unchanged.",
    sourceUrl: SRC.dlssOverview,
  },
  {
    value: "fixed",
    label: "Fixed multiplier",
    description:
      "Force a fixed multiplier (2×/3×/4×, up to 6× in supported titles). 'App controlled' count keeps the in-game 2×.",
    sourceUrl: SRC.dynamicMfg,
  },
  {
    value: "dynamic",
    label: "Dynamic (DLSS 4.5)",
    description:
      "Automatically shifts the multiplier to hit your target frame rate / refresh rate. RTX 50 only. Not compatible with frame-rate limiters or V-Sync.",
    sourceUrl: SRC.dynamicMfg,
  },
];

export const FG_COUNT_OPTIONS: Option<FrameGenCount>[] = [
  {
    value: "app_controlled",
    label: "App controlled",
    description: "Let the game/driver choose the Frame Generation multiplier.",
    sourceUrl: SRC.dlssOverview,
  },
  {
    value: "x2",
    label: "2× Frame Generation",
    description: "One generated frame per rendered frame. Supported on RTX 40 and RTX 50.",
    sourceUrl: SRC.dynamicMfg,
  },
  {
    value: "x3",
    label: "3× Multi Frame Generation",
    description: "Two generated frames per rendered frame. RTX 50 only.",
    sourceUrl: SRC.dynamicMfg,
  },
  {
    value: "x4",
    label: "4× Multi Frame Generation",
    description: "Three generated frames per rendered frame. RTX 50 only.",
    sourceUrl: SRC.dynamicMfg,
  },
];

export const DLSS4_MIN_DRIVER_PACKED = 57216;
export const DYNAMIC_MFG_MIN_DRIVER_PACKED = 59597;

export function emptyDlssConfig(): DlssOverrideConfig {
  return {
    enable_sr_dll_override: false,
    sr_preset: null,
    enable_fg_dll_override: false,
    fg_preset: null,
    fg_mode: null,
    fg_fixed_count: null,
    fg_dynamic_target_fps: null,
  };
}

export function presetLabel(preset: DlssPreset): string {
  return SR_PRESET_OPTIONS.find((option) => option.value === preset)?.label ?? preset.toUpperCase();
}

export function dlss4Available(driverPacked: number): boolean {
  return driverPacked >= DLSS4_MIN_DRIVER_PACKED;
}

export function dynamicMfgAvailable(driverPacked: number): boolean {
  return driverPacked >= DYNAMIC_MFG_MIN_DRIVER_PACKED;
}

export function hasActiveOverride(config: DlssOverrideConfig): boolean {
  return (
    config.enable_sr_dll_override ||
    config.enable_fg_dll_override ||
    config.sr_preset != null ||
    config.fg_preset != null ||
    config.fg_mode != null ||
    config.fg_fixed_count != null ||
    config.fg_dynamic_target_fps != null
  );
}
