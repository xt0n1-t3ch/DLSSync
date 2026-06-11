export const EFFICIENCY_PREF_KEY = "dlssync-pref-efficiency-mode";
export const LEGACY_EFFICIENCY_PREF_KEY = "dlssync-pref-efficiency-on-minimize";
export const EFFICIENCY_PREF_EVENT = "dlssync:efficiency-mode";

export type EfficiencyPreferenceEvent = CustomEvent<{ enabled: boolean }>;

/// Read the efficiency preference from localStorage, defaulting to OFF on first run.
///
/// - If `EFFICIENCY_PREF_KEY` is set, parse it: "false" → false, any other value → true.
/// - Otherwise, if `LEGACY_EFFICIENCY_PREF_KEY` is set, parse it: "true" → true, any other value → false.
/// - Otherwise (both keys absent, first run) → false (opt-in, not opt-out).
export function readEfficiencyPreference(): boolean {
  const current = localStorage.getItem(EFFICIENCY_PREF_KEY);
  const legacy = localStorage.getItem(LEGACY_EFFICIENCY_PREF_KEY);
  return current !== null ? current !== "false" : legacy === "true";
}

export function writeEfficiencyPreference(enabled: boolean): void {
  localStorage.setItem(EFFICIENCY_PREF_KEY, String(enabled));
  window.dispatchEvent(new CustomEvent(EFFICIENCY_PREF_EVENT, { detail: { enabled } }));
}
