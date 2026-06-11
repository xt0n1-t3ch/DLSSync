import type { BackupEntry, DetectedGame, DllRecord } from "./api";

export interface RevertedSwap {
  game: DetectedGame;
  record: DllRecord;
  backup: BackupEntry;
}

/** A game update silently rolled a swapped DLL back when the newest
 *  non-restored backup for a path recorded `previous_version` X (the version
 *  DLSSync replaced) and the file on disk reads X again: DLSSync wrote
 *  something newer over X, so X reappearing means the game clobbered the swap.
 *  A manual restore sets `restored_at` and is excluded; a re-apply records a
 *  fresh backup whose `previous_version` no longer matches the disk. */
export function detectRevertedSwaps(
  games: DetectedGame[],
  dllsByGame: Record<string, DllRecord[]>,
  backups: BackupEntry[],
): RevertedSwap[] {
  const newestByPath = new Map<string, BackupEntry>();
  for (const backup of backups) {
    if (backup.restored_at !== null) continue;
    if (backup.backup_type === "driver_package") continue;
    if (!backup.previous_version) continue;
    const key = normalizePath(backup.original_path);
    const current = newestByPath.get(key);
    if (!current || backup.created_at > current.created_at) newestByPath.set(key, backup);
  }
  if (newestByPath.size === 0) return [];

  const reverted: RevertedSwap[] = [];
  for (const game of games) {
    for (const record of dllsByGame[game.id] ?? []) {
      const backup = newestByPath.get(normalizePath(record.path));
      if (!backup) continue;
      if (record.current_version !== null && record.current_version === backup.previous_version) {
        reverted.push({ game, record, backup });
      }
    }
  }
  return reverted;
}

function normalizePath(path: string): string {
  return path.replace(/\//g, "\\").toLowerCase();
}
