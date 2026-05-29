import {
  siAmd,
  siAsus,
  siBroadcom,
  siDell,
  siIntel,
  siMediatek,
  siMsi,
  siNvidia,
  siQualcomm,
  siRazer,
} from "simple-icons";

export interface Brand {
  label: string;
  path: string;
  viewBox: string;
  accentVar: string;
}

const VENDOR_NVIDIA = "--vendor-nvidia";
const VENDOR_AMD = "--vendor-amd";
const VENDOR_INTEL = "--vendor-intel";
const VENDOR_MICROSOFT = "--vendor-microsoft";
const VENDOR_NEUTRAL = "--neutral";

const SQUARE_24 = "0 0 24 24";

const MICROSOFT_SQUARES = "M11.4 11.4H0V0h11.4zM24 11.4H12.6V0H24zM11.4 24H0V12.6h11.4zM24 24H12.6V12.6H24z";

export const BRANDS = {
  nvidia: { label: "NVIDIA", viewBox: SQUARE_24, accentVar: VENDOR_NVIDIA, path: siNvidia.path },
  amd: { label: "AMD", viewBox: SQUARE_24, accentVar: VENDOR_AMD, path: siAmd.path },
  intel: { label: "Intel", viewBox: SQUARE_24, accentVar: VENDOR_INTEL, path: siIntel.path },
  microsoft: { label: "Microsoft", viewBox: SQUARE_24, accentVar: VENDOR_MICROSOFT, path: MICROSOFT_SQUARES },
  dell: { label: "Dell", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siDell.path },
  msi: { label: "MSI", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siMsi.path },
  asus: { label: "ASUS", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siAsus.path },
  qualcomm: { label: "Qualcomm", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siQualcomm.path },
  razer: { label: "Razer", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siRazer.path },
  broadcom: { label: "Broadcom", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siBroadcom.path },
  mediatek: { label: "MediaTek", viewBox: SQUARE_24, accentVar: VENDOR_NEUTRAL, path: siMediatek.path },
} as const satisfies Record<string, Brand>;

export type BrandKey = keyof typeof BRANDS;

const BRAND_KEY_MATCHERS: ReadonlyArray<readonly [RegExp, BrandKey]> = [
  [/advanced micro devices|\bati\b|radeon|\bamd\b/, "amd"],
  [/\bnvidia\b|geforce|\brtx\b|\bgtx\b/, "nvidia"],
  [/\bintel\b/, "intel"],
  [/microsoft|\bmsft\b/, "microsoft"],
  [/\bdell\b|alienware/, "dell"],
  [/micro-?star|\bmsi\b/, "msi"],
  [/\basus\b|asustek|\brog\b/, "asus"],
  [/qualcomm|atheros|\bqca\b|snapdragon/, "qualcomm"],
  [/\brazer\b/, "razer"],
  [/broadcom|\bbcm\b/, "broadcom"],
  [/mediatek|\bmtk\b/, "mediatek"],
];

const BRAND_DOMAIN_MATCHERS: ReadonlyArray<readonly [RegExp, string]> = [
  [/realtek|\brtk\b/, "realtek.com"],
  [/a-?volute|nahimic/, "nahimic.com"],
  [/synaptics/, "synaptics.com"],
  [/gigabyte|gig-?a-?byte|\baorus\b/, "gigabyte.com"],
  [/logitech|\blogi\b/, "logitech.com"],
  [/sound ?blaster|creative (labs|technology|technologies)/, "creative.com"],
  [/elan(tech| microelectronics)?/, "emc.com.tw"],
  [/conexant/, "synaptics.com"],
  [/hewlett-?packard|\bhp\b/, "hp.com"],
  [/lenovo/, "lenovo.com"],
  [/\bacer\b/, "acer.com"],
  [/advanced micro devices|radeon|\bati\b|\bamd\b/, "amd.com"],
  [/\bnvidia\b|geforce/, "nvidia.com"],
  [/\bintel\b/, "intel.com"],
  [/microsoft|\bmsft\b/, "microsoft.com"],
  [/\bdell\b|alienware/, "dell.com"],
  [/micro-?star|\bmsi\b/, "msi.com"],
  [/\basus\b|asustek|\brog\b/, "asus.com"],
  [/qualcomm|atheros|snapdragon/, "qualcomm.com"],
  [/\brazer\b/, "razer.com"],
  [/broadcom|\bbcm\b/, "broadcom.com"],
  [/mediatek|\bmtk\b/, "mediatek.com"],
];

export function resolveBrandKey(raw: string | null | undefined): BrandKey | null {
  if (!raw) return null;
  const normalized = raw.toLowerCase();
  if (normalized in BRANDS) return normalized as BrandKey;
  for (const [matcher, key] of BRAND_KEY_MATCHERS) {
    if (matcher.test(normalized)) return key;
  }
  return null;
}

export function resolveBrandDomain(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const normalized = raw.toLowerCase();
  for (const [matcher, domain] of BRAND_DOMAIN_MATCHERS) {
    if (matcher.test(normalized)) return domain;
  }
  return null;
}

function brandfetchClientId(): string {
  const env = import.meta.env as Record<string, string | undefined>;
  return env?.VITE_BRANDFETCH_CLIENT_ID ?? "";
}

export function brandfetchConfigured(): boolean {
  return brandfetchClientId().length > 0;
}

export interface BrandfetchOptions {
  size?: number;
  clientId?: string;
}

export function brandfetchLogoUrl(
  domain: string | null | undefined,
  options: BrandfetchOptions = {},
): string | null {
  const clientId = options.clientId ?? brandfetchClientId();
  if (!domain || !clientId) return null;
  const size = Math.max(16, Math.round(options.size ?? 64));
  return `https://cdn.brandfetch.io/${domain}/w/${size}/h/${size}/icon?c=${clientId}`;
}

export function brandLabel(raw: string): string {
  const key = resolveBrandKey(raw);
  return key ? BRANDS[key].label : raw;
}

export function brandFor(raw: string | null | undefined): Brand | null {
  const key = resolveBrandKey(raw);
  return key ? BRANDS[key] : null;
}
