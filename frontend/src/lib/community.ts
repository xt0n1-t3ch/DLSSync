import { get, writable } from "svelte/store";
import { EXTERNAL_URLS } from "./ux";
import { settings, persistSettings } from "./stores";
import { isNexusBuild } from "./distribution";

const STAR_CACHE_KEY = "dlssync.starCount.v1";
const STAR_TTL_MS = 6 * 60 * 60 * 1000;
const STAR_ENDPOINT = "https://api.github.com/repos/xt0n1-t3ch/DLSSync";

/** True while the support card is on screen. It persists until the user dismisses
 * it or chooses "Don't show again" - it never auto-times-out. */
export const supportNudgeVisible = writable(false);

/** Once shown in a session, it does not re-pop on every later apply (avoids nagging).
 * It can show again next session, unless the user turned it off for good. */
let shownThisSession = false;

export const SHARE_TEXT =
  "DLSSync - a free, open-source app that keeps DLSS, FSR, XeSS, frame generation and your GPU drivers up to date on Windows.";

export function nudgeEnabled(): boolean {
  return get(settings)?.ui_prefs.show_support_nudge ?? true;
}

/** Surface the support card after a successful apply (the value moment), at most
 * once per session, and only when the user has not turned it off. */
export function notifyApplySuccess(successCount: number): void {
  if (successCount <= 0 || shownThisSession || !nudgeEnabled()) return;
  shownThisSession = true;
  supportNudgeVisible.set(true);
}

/** Close the card for now. It may appear again at a future value moment / session. */
export function dismissNudge(): void {
  supportNudgeVisible.set(false);
}

/** Turn the card off for good (persisted in settings; re-enable in Settings). */
export async function dontShowAgain(): Promise<void> {
  supportNudgeVisible.set(false);
  const current = get(settings);
  if (current && current.ui_prefs.show_support_nudge) {
    await persistSettings({
      ...current,
      ui_prefs: { ...current.ui_prefs, show_support_nudge: false },
    });
  }
}

/** Let the card surface again this session (called when re-enabled in Settings). */
export function resetNudgeSession(): void {
  shownThisSession = false;
}

export type ShareResult = "shared" | "copied" | "failed";

export async function shareDlssync(): Promise<ShareResult> {
  // The Nexus build shares the Nexus mod page, never the GitHub repo.
  const url = isNexusBuild ? EXTERNAL_URLS.nexusMod : EXTERNAL_URLS.homepage;
  const message = `${SHARE_TEXT} ${url}`;
  const nav = navigator as Navigator & {
    share?: (data: { title: string; text: string; url: string }) => Promise<void>;
  };
  if (typeof nav.share === "function") {
    try {
      await nav.share({ title: "DLSSync", text: SHARE_TEXT, url });
      return "shared";
    } catch {
      /* user cancelled or unsupported - fall back to clipboard */
    }
  }
  try {
    await navigator.clipboard.writeText(message);
    return "copied";
  } catch {
    return "failed";
  }
}

interface StarCache {
  count: number;
  at: number;
}

/** Live GitHub star count for social proof. Cached in localStorage for STAR_TTL_MS
 * (GitHub allows 60 unauthenticated req/h per IP). Returns null on any failure. */
export async function fetchStarCount(now: number = Date.now()): Promise<number | null> {
  // The Nexus build never calls the GitHub API.
  if (isNexusBuild) return null;
  try {
    const raw = localStorage.getItem(STAR_CACHE_KEY);
    if (raw) {
      const cached = JSON.parse(raw) as StarCache;
      if (typeof cached.count === "number" && now - cached.at < STAR_TTL_MS) {
        return cached.count;
      }
    }
  } catch {
    /* ignore malformed cache */
  }
  try {
    const res = await fetch(STAR_ENDPOINT, { headers: { Accept: "application/vnd.github+json" } });
    if (!res.ok) return null;
    const data = (await res.json()) as { stargazers_count?: unknown };
    const count = typeof data.stargazers_count === "number" ? data.stargazers_count : null;
    if (count !== null) {
      try {
        localStorage.setItem(STAR_CACHE_KEY, JSON.stringify({ count, at: now } satisfies StarCache));
      } catch {
        /* storage unavailable - skip caching */
      }
    }
    return count;
  } catch {
    return null;
  }
}
