import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { settings } from "@/lib/stores";
import {
  supportNudgeVisible,
  notifyApplySuccess,
  dismissNudge,
  resetNudgeSession,
  nudgeEnabled,
  shareDlssync,
  fetchStarCount,
  SHARE_TEXT,
} from "@/lib/community";
import type { AppSettings } from "@/lib/api";

function withNudge(show: boolean): AppSettings {
  return { ui_prefs: { show_support_nudge: show } } as unknown as AppSettings;
}

beforeEach(() => {
  settings.set(null);
  supportNudgeVisible.set(false);
  resetNudgeSession();
  localStorage.clear();
});

describe("support nudge gating", () => {
  it("does not show when nothing succeeded", () => {
    settings.set(withNudge(true));
    notifyApplySuccess(0);
    expect(get(supportNudgeVisible)).toBe(false);
  });

  it("does not show when the user turned it off", () => {
    settings.set(withNudge(false));
    notifyApplySuccess(2);
    expect(get(supportNudgeVisible)).toBe(false);
  });

  it("shows once after a successful apply when enabled", () => {
    settings.set(withNudge(true));
    notifyApplySuccess(1);
    expect(get(supportNudgeVisible)).toBe(true);
  });

  it("does not re-pop in the same session after a dismiss", () => {
    settings.set(withNudge(true));
    notifyApplySuccess(1);
    dismissNudge();
    expect(get(supportNudgeVisible)).toBe(false);
    notifyApplySuccess(3);
    expect(get(supportNudgeVisible)).toBe(false);
  });

  it("can surface again once the session is reset (re-enabled in Settings)", () => {
    settings.set(withNudge(true));
    notifyApplySuccess(1);
    dismissNudge();
    resetNudgeSession();
    notifyApplySuccess(1);
    expect(get(supportNudgeVisible)).toBe(true);
  });

  it("defaults to enabled when settings are not loaded yet", () => {
    expect(nudgeEnabled()).toBe(true);
  });
});

describe("shareDlssync", () => {
  it("copies a link containing the share text when Web Share is unavailable", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "share", { value: undefined, configurable: true });
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    const result = await shareDlssync();
    expect(result).toBe("copied");
    expect(writeText).toHaveBeenCalledOnce();
    expect(String(writeText.mock.calls[0][0])).toContain(SHARE_TEXT);
  });
});

describe("fetchStarCount", () => {
  it("returns a fresh cached count without hitting the network", async () => {
    localStorage.setItem("dlssync.starCount.v1", JSON.stringify({ count: 42, at: 1000 }));
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    const count = await fetchStarCount(1000 + 1000);
    expect(count).toBe(42);
    expect(fetchSpy).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("fetches and returns the live count when there is no fresh cache", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: true, json: async () => ({ stargazers_count: 7 }) }),
    );
    const count = await fetchStarCount(5000);
    expect(count).toBe(7);
    vi.unstubAllGlobals();
  });

  it("returns null when the request fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false }));
    const count = await fetchStarCount(5000);
    expect(count).toBeNull();
    vi.unstubAllGlobals();
  });
});
