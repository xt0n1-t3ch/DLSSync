// Pure favorites logic. The single owner of the data is Rust's
// UiPreferences.favorite_game_ids (see src-tauri/src/commands/settings.rs); this
// module holds the side-effect-free list operations so they are unit-tested once
// and reused by the store action and every favorite affordance.

/** True when `id` is in the favorites list. */
export function isFavorite(ids: readonly string[], id: string): boolean {
  return ids.includes(id);
}

/** Toggle `id`: append when absent, remove when present. Preserves the order of
 *  the remaining ids, never duplicates, ignores an empty id, and never mutates
 *  the input. */
export function toggleFavorite(ids: readonly string[], id: string): string[] {
  if (!id) return [...ids];
  return ids.includes(id) ? ids.filter((existing) => existing !== id) : [...ids, id];
}

/** Keep only entries whose id is in the favorites set, preserving input order. */
export function filterFavorites<T extends { id: string }>(
  items: readonly T[],
  ids: ReadonlySet<string>,
): T[] {
  return items.filter((item) => ids.has(item.id));
}
