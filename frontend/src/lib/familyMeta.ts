import type { DllFamily } from "./api";

/** The four hardware/platform vendors DLSSync routes DLL families to.
 *  Load-bearing: the vendor decides CDN routing for downloads, so it must be
 *  exactly one value per family and agree everywhere it is read. */
export type VendorKey = "nvidia" | "amd" | "intel" | "microsoft";

/** Canonical per-family metadata. Every {@link DllFamily} resolves here to a
 *  single vendor plus the CSS custom-property token used to tint that vendor.
 *
 *  This record is the ONE source of truth: `ux.ts` (`vendorForFamily`,
 *  `VENDOR_TOKEN_BY_FAMILY`) and `labels.ts` (`FAMILY_TO_VENDOR`) all delegate
 *  here so they can never drift apart again.
 *
 *  Two families historically disagreed across the old tables:
 *  - `streamline_direct_sr` is Microsoft DirectSR wrapped in an NVIDIA
 *    Streamline plug-in. The upstream technology (and therefore the download
 *    source) is Microsoft, so the vendor is `microsoft`.
 *  - `direct_storage_core` is a Microsoft DirectStorage component
 *    (`startsWith("direct_storage")`) and resolves to `microsoft`. */
export interface FamilyMeta {
  vendor: VendorKey;
  /** CSS custom-property reference, e.g. `var(--vendor-nvidia)`. */
  token: string;
}

function tokenFor(vendor: VendorKey): string {
  return `var(--vendor-${vendor})`;
}

function meta(vendor: VendorKey): FamilyMeta {
  return { vendor, token: tokenFor(vendor) };
}

export const FAMILY_META: Record<DllFamily, FamilyMeta> = {
  dlss_sr: meta("nvidia"),
  dlss_fg: meta("nvidia"),
  dlss_rr: meta("nvidia"),
  sl_dlss_sr: meta("nvidia"),
  sl_dlss_fg: meta("nvidia"),
  sl_dlss_rr: meta("nvidia"),
  streamline: meta("nvidia"),
  streamline_common: meta("nvidia"),
  streamline_pcl: meta("nvidia"),
  streamline_nis: meta("nvidia"),
  streamline_direct_sr: meta("microsoft"),
  reflex: meta("nvidia"),
  xess_sr: meta("intel"),
  xess_sr_dx11: meta("intel"),
  xess_fg: meta("intel"),
  xell: meta("intel"),
  fsr_upscaler: meta("amd"),
  fsr_upscaler_vk: meta("amd"),
  fsr_fg: meta("amd"),
  fsr_loader: meta("amd"),
  fsr_denoiser: meta("amd"),
  direct_storage: meta("microsoft"),
  direct_storage_core: meta("microsoft"),
};

/** Resolve a family string to its canonical {@link FamilyMeta}, or `null` when
 *  the string is not a known {@link DllFamily}. Accepts an arbitrary string so
 *  callers can pass values straight from records without a cast. */
export function familyMeta(family: string): FamilyMeta | null {
  return (FAMILY_META as Record<string, FamilyMeta | undefined>)[family] ?? null;
}
