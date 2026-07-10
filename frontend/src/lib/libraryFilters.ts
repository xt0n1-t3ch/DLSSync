// Pure predicates for the advanced Library filters (technology/vendor-family and
// anti-cheat risk). Driven by real scanned DLL data and the anti-cheat report set
// — never fixture-only logic. Unit-tested here once; the store wires them into the
// filteredGames derivation alongside launcher/status/favorite.
import { familyGroup, type FamilyGroup } from "./labels";

export type TechnologyFilter = "all" | FamilyGroup;
export type AntiCheatFilter = "all" | "flagged" | "clear";

/** The DLL family-groups (dlss/fsr/xess/advanced) present in a game's scanned DLLs. */
export function gameTechnologies(dlls: readonly { family: string }[]): Set<FamilyGroup> {
  const out = new Set<FamilyGroup>();
  for (const record of dlls) out.add(familyGroup(record.family));
  return out;
}

/** True when the game has a DLL in the selected technology group (or filter is "all"). */
export function matchesTechnology(
  dlls: readonly { family: string }[],
  filter: TechnologyFilter,
): boolean {
  if (filter === "all") return true;
  return gameTechnologies(dlls).has(filter);
}

/** True when the game's anti-cheat state matches the filter. `flagged` = present in
 *  the detected-anti-cheat set; `clear` = absent. */
export function matchesAntiCheat(
  gameId: string,
  flagged: ReadonlySet<string>,
  filter: AntiCheatFilter,
): boolean {
  if (filter === "all") return true;
  const isFlagged = flagged.has(gameId);
  return filter === "flagged" ? isFlagged : !isFlagged;
}
