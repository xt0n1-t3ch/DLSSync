const KIB = 1024;
const MIB = 1024 * 1024;
const GIB = 1024 * 1024 * 1024;

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) return "—";
  const n = Math.max(0, bytes);
  if (n >= GIB) return `${(n / GIB).toFixed(2)} GB`;
  if (n >= MIB) return `${(n / MIB).toFixed(1)} MB`;
  if (n >= KIB) return `${(n / KIB).toFixed(0)} KB`;
  return `${n} B`;
}

export function formatSpeed(bytesPerSec: number | null | undefined): string {
  if (!bytesPerSec || !Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return "—";
  return `${formatBytes(bytesPerSec)}/s`;
}

export function formatEta(
  bytesDownloaded: number,
  bytesTotal: number | null | undefined,
  bytesPerSec: number,
): string {
  if (!bytesTotal || !bytesPerSec || bytesPerSec <= 0) return "—";
  const remaining = Math.max(0, bytesTotal - bytesDownloaded);
  if (remaining === 0) return "0s";
  const secs = Math.ceil(remaining / bytesPerSec);
  return formatDurationSecs(secs);
}

export function formatDurationSecs(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return "—";
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) {
    const m = Math.floor(secs / 60);
    const s = Math.round(secs % 60);
    return s === 0 ? `${m}m` : `${m}m ${s}s`;
  }
  const h = Math.floor(secs / 3600);
  const m = Math.round((secs % 3600) / 60);
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

export function formatElapsedSince(startedAt: number, endedAt: number | null): string {
  const now = endedAt ?? Date.now();
  const secs = Math.max(0, Math.floor((now - startedAt) / 1000));
  return formatDurationSecs(secs);
}

export function percentOf(numer: number, denom: number | null | undefined): number {
  if (!denom || denom <= 0) return 0;
  return Math.min(100, Math.max(0, (numer / denom) * 100));
}
