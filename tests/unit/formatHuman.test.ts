import { describe, it, expect } from "vitest";
import {
  formatBytes,
  formatSpeed,
  formatEta,
  formatDurationSecs,
  formatElapsedSince,
  percentOf,
} from "@/lib/formatHuman";

describe("formatBytes", () => {
  it("renders byte/KB/MB/GB tiers with documented precision", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("2 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
    expect(formatBytes(5 * 1024 * 1024 + 512 * 1024)).toBe("5.5 MB");
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.00 GB");
    expect(formatBytes(2.5 * 1024 * 1024 * 1024)).toBe("2.50 GB");
  });

  it("null/undefined/NaN/Infinity collapse to em dash", () => {
    expect(formatBytes(null)).toBe("—");
    expect(formatBytes(undefined)).toBe("—");
    expect(formatBytes(NaN)).toBe("—");
    expect(formatBytes(Infinity)).toBe("—");
  });

  it("clamps negatives to zero floor", () => {
    expect(formatBytes(-100)).toBe("0 B");
  });
});

describe("formatSpeed", () => {
  it("appends /s to a byte rate", () => {
    expect(formatSpeed(1024)).toBe("1 KB/s");
    expect(formatSpeed(1024 * 1024)).toBe("1.0 MB/s");
  });

  it("zero/negative/null/NaN collapse to em dash", () => {
    expect(formatSpeed(0)).toBe("—");
    expect(formatSpeed(-5)).toBe("—");
    expect(formatSpeed(null)).toBe("—");
    expect(formatSpeed(NaN)).toBe("—");
  });
});

describe("formatDurationSecs", () => {
  it("seconds under a minute", () => {
    expect(formatDurationSecs(0)).toBe("0s");
    expect(formatDurationSecs(45)).toBe("45s");
    expect(formatDurationSecs(59.6)).toBe("60s");
  });

  it("minutes drop the seconds when whole", () => {
    expect(formatDurationSecs(60)).toBe("1m");
    expect(formatDurationSecs(90)).toBe("1m 30s");
    expect(formatDurationSecs(600)).toBe("10m");
  });

  it("hours drop the minutes when whole", () => {
    expect(formatDurationSecs(3600)).toBe("1h");
    expect(formatDurationSecs(3600 + 1800)).toBe("1h 30m");
  });

  it("negative/NaN collapse to em dash", () => {
    expect(formatDurationSecs(-1)).toBe("—");
    expect(formatDurationSecs(NaN)).toBe("—");
  });
});

describe("formatEta", () => {
  it("computes remaining time from rate", () => {
    expect(formatEta(0, 1000, 100)).toBe("10s");
    expect(formatEta(900, 1000, 100)).toBe("1s");
  });

  it("zero remaining is 0s", () => {
    expect(formatEta(1000, 1000, 100)).toBe("0s");
  });

  it("unknown total or zero rate collapses to em dash", () => {
    expect(formatEta(0, null, 100)).toBe("—");
    expect(formatEta(0, 1000, 0)).toBe("—");
  });

  it("downloaded beyond total never goes negative", () => {
    expect(formatEta(2000, 1000, 100)).toBe("0s");
  });
});

describe("formatElapsedSince", () => {
  it("uses endedAt when provided", () => {
    const start = 1_000_000;
    expect(formatElapsedSince(start, start + 90_000)).toBe("1m 30s");
  });

  it("clamps to zero when clock skews backwards", () => {
    expect(formatElapsedSince(2_000_000, 1_000_000)).toBe("0s");
  });

  it("falls back to now when endedAt is null", () => {
    const out = formatElapsedSince(Date.now(), null);
    expect(out).toMatch(/^\d+s$/);
  });
});

describe("percentOf", () => {
  it("computes a clamped percentage", () => {
    expect(percentOf(50, 100)).toBe(50);
    expect(percentOf(0, 100)).toBe(0);
    expect(percentOf(200, 100)).toBe(100);
  });

  it("zero/null/negative denominator is zero", () => {
    expect(percentOf(5, 0)).toBe(0);
    expect(percentOf(5, null)).toBe(0);
    expect(percentOf(5, -10)).toBe(0);
  });
});
