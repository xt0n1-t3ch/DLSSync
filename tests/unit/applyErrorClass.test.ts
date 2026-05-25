import { describe, it, expect } from "vitest";
import {
  classifyApplyError,
  classOf,
  ERROR_CLASS_LABEL,
  ERROR_CLASS_TONE,
} from "@/lib/applyErrorClass";
import type { ApplyErrorClass } from "@/lib/api";

const ALL_CLASSES: ApplyErrorClass[] = [
  "network",
  "signature",
  "lock",
  "permission",
  "hash",
  "missing",
  "backup",
  "cancelled",
  "other",
];

describe("classifyApplyError", () => {
  it("empty/null/whitespace is 'other' with no action", () => {
    for (const m of [null, undefined, "", "   "]) {
      const c = classifyApplyError(m);
      expect(c.kind).toBe("other");
      expect(c.action).toBe("none");
      expect(c.retryable).toBe(false);
    }
  });

  it("cancelled is checked before network", () => {
    expect(classOf("Cancelled by user")).toBe("cancelled");
  });

  it("network needles classify as network/retry", () => {
    for (const m of ["error sending request", "connection reset by peer", "request timed out", "HTTP 503", "size mismatch"]) {
      const c = classifyApplyError(m);
      expect(c.kind).toBe("network");
      expect(c.action).toBe("retry");
    }
  });

  it("signature needles route to allow_unsigned_and_retry", () => {
    const c = classifyApplyError("CRYPT_E_NO_MATCH while verifying");
    expect(c.kind).toBe("signature");
    expect(c.action).toBe("allow_unsigned_and_retry");
  });

  it("lock needles route to close_game_and_retry", () => {
    expect(classifyApplyError("file is locked by another process").action).toBe("close_game_and_retry");
    expect(classOf("SHARING_VIOLATION")).toBe("lock");
  });

  it("permission needles are non-retryable elevate", () => {
    const c = classifyApplyError("Access denied (os error 5)");
    expect(c.kind).toBe("permission");
    expect(c.retryable).toBe(false);
    expect(c.action).toBe("elevate");
  });

  it("hash needles route to refresh_catalog", () => {
    expect(classifyApplyError("SHA-256 mismatch on payload").action).toBe("refresh_catalog");
  });

  it("missing needles are non-retryable report", () => {
    const c = classifyApplyError("nvngx_dlss.dll not in zip");
    expect(c.kind).toBe("missing");
    expect(c.action).toBe("report");
    expect(c.retryable).toBe(false);
  });

  it("backup needles route to retry", () => {
    expect(classOf("backup store write failed")).toBe("backup");
  });

  it("unclassified message is 'other' but retryable", () => {
    const c = classifyApplyError("something totally unexpected happened");
    expect(c.kind).toBe("other");
    expect(c.retryable).toBe(true);
    expect(c.action).toBe("retry");
  });

  it("classification is case-insensitive", () => {
    expect(classOf("ACCESS DENIED")).toBe("permission");
    expect(classOf("Connection Refused")).toBe("network");
  });
});

describe("error class tables", () => {
  it("every class has a label and a tone", () => {
    for (const k of ALL_CLASSES) {
      expect(ERROR_CLASS_LABEL[k]).toBeTruthy();
      expect(["danger", "warning", "info", "neutral"]).toContain(ERROR_CLASS_TONE[k]);
    }
  });
});
