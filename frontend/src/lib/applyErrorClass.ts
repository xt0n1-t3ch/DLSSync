import type { ApplyErrorClass } from "./api";

export interface ClassifiedError {
  kind: ApplyErrorClass;
  short: string;
  hint: string;
  retryable: boolean;
  action: "retry" | "allow_unsigned_and_retry" | "close_game_and_retry" | "elevate" | "refresh_catalog" | "report" | "none";
}

const NETWORK_NEEDLES = [
  "error sending request",
  "decoding response body",
  "connection reset",
  "connection refused",
  "stalled",
  "truncated",
  "timed out",
  "dns",
  "size mismatch",
  "429",
  "503",
  "502",
  "504",
];

const SIGNATURE_NEEDLES = [
  "crypt_e_no_match",
  "allow unsigned",
  "authenticode signature could not be read",
  "no authenticode subject",
  "allowlist",
];

const LOCK_NEEDLES = ["locked by another process", "sharing_violation"];
const PERMISSION_NEEDLES = ["access denied", "administrator"];
const HASH_NEEDLES = ["sha-256 mismatch", "integrity", "md5 mismatch"];
const MISSING_NEEDLES = ["not in zip", "dll not found", "release ", "not in catalog"];
const BACKUP_NEEDLES = ["backup"];
const CANCELLED_NEEDLES = ["cancelled by user", "cancelled"];

export function classifyApplyError(message: string | null | undefined): ClassifiedError {
  const raw = (message ?? "").trim();
  const lower = raw.toLowerCase();
  if (!raw) {
    return {
      kind: "other",
      short: "Unknown error",
      hint: "No error message reported by the backend.",
      retryable: false,
      action: "none",
    };
  }
  if (anyMatch(lower, CANCELLED_NEEDLES)) {
    return {
      kind: "cancelled",
      short: "Cancelled",
      hint: "You cancelled this apply. Click Retry to try again.",
      retryable: true,
      action: "retry",
    };
  }
  if (anyMatch(lower, NETWORK_NEEDLES)) {
    return {
      kind: "network",
      short: "Network hiccup",
      hint: "The vendor CDN flaked mid-download. Retrying usually fixes it.",
      retryable: true,
      action: "retry",
    };
  }
  if (anyMatch(lower, SIGNATURE_NEEDLES)) {
    return {
      kind: "signature",
      short: "Unsigned DLL or vendor mismatch",
      hint: "Vendor ships this DLL unsigned, or the embedded subject is not on the allowlist. Toggle 'Allow unsigned DLLs' in Settings → Advanced and retry — SHA-256 is still enforced.",
      retryable: true,
      action: "allow_unsigned_and_retry",
    };
  }
  if (anyMatch(lower, LOCK_NEEDLES)) {
    return {
      kind: "lock",
      short: "Game has the DLL open",
      hint: "Close the game (and the launcher's anti-cheat helper if applicable), then retry.",
      retryable: true,
      action: "close_game_and_retry",
    };
  }
  if (anyMatch(lower, PERMISSION_NEEDLES)) {
    return {
      kind: "permission",
      short: "Permission denied",
      hint: "Run DLSSync as Administrator to write into this install directory, then retry.",
      retryable: false,
      action: "elevate",
    };
  }
  if (anyMatch(lower, HASH_NEEDLES)) {
    return {
      kind: "hash",
      short: "Integrity mismatch",
      hint: "The downloaded DLL did not match the manifest SHA-256. The manifest may be stale — Refresh catalog and retry.",
      retryable: true,
      action: "refresh_catalog",
    };
  }
  if (anyMatch(lower, MISSING_NEEDLES)) {
    return {
      kind: "missing",
      short: "Asset missing",
      hint: "The expected file was not in the vendor archive (or the release was withdrawn). Report this so the catalog can be patched.",
      retryable: false,
      action: "report",
    };
  }
  if (anyMatch(lower, BACKUP_NEEDLES)) {
    return {
      kind: "backup",
      short: "Backup store unavailable",
      hint: "Backup database or folder could not be written. Check disk space and that no other DLSSync instance is running.",
      retryable: true,
      action: "retry",
    };
  }
  return {
    kind: "other",
    short: "Unexpected error",
    hint: "An unclassified failure occurred. Use 'Copy report' and open an issue if it persists.",
    retryable: true,
    action: "retry",
  };
}

function anyMatch(haystack: string, needles: string[]): boolean {
  return needles.some((n) => haystack.includes(n));
}

export function classOf(message: string | null | undefined): ApplyErrorClass {
  return classifyApplyError(message).kind;
}

export const ERROR_CLASS_LABEL: Record<ApplyErrorClass, string> = {
  network: "Network",
  signature: "Signature",
  lock: "Locked",
  permission: "Permission",
  hash: "Integrity",
  missing: "Missing",
  backup: "Backup",
  cancelled: "Cancelled",
  other: "Unknown",
};

export const ERROR_CLASS_TONE: Record<ApplyErrorClass, "danger" | "warning" | "info" | "neutral"> = {
  network: "warning",
  signature: "warning",
  lock: "info",
  permission: "danger",
  hash: "danger",
  missing: "danger",
  backup: "danger",
  cancelled: "neutral",
  other: "danger",
};
