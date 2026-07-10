import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import type { AppSettings } from "@/lib/api";

const addFavoriteGame = vi.fn<(id: string) => Promise<string[]>>();
const removeFavoriteGame = vi.fn<(id: string) => Promise<string[]>>();

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    addFavoriteGame: (id: string) => addFavoriteGame(id),
    removeFavoriteGame: (id: string) => removeFavoriteGame(id),
  };
});

import { settings, favoriteIds, toggleFavorite } from "@/lib/stores";

function baseSettings(favorites: string[] = []): AppSettings {
  return {
    ui_prefs: { favorite_game_ids: [...favorites] },
    blacklist: [],
  } as unknown as AppSettings;
}

describe("toggleFavorite atomic backend (no lost update)", () => {
  beforeEach(() => {
    addFavoriteGame.mockReset();
    removeFavoriteGame.mockReset();
    settings.set(baseSettings([]));
  });

  it("favorites a game via add_favorite_game and reflects the authoritative list", async () => {
    addFavoriteGame.mockResolvedValue(["g1"]);
    await toggleFavorite("g1");
    expect(addFavoriteGame).toHaveBeenCalledWith("g1");
    expect(removeFavoriteGame).not.toHaveBeenCalled();
    expect(get(favoriteIds)).toEqual(new Set(["g1"]));
  });

  it("unfavorites an already-favorited game via remove_favorite_game", async () => {
    settings.set(baseSettings(["g1"]));
    removeFavoriteGame.mockResolvedValue([]);
    await toggleFavorite("g1");
    expect(removeFavoriteGame).toHaveBeenCalledWith("g1");
    expect(addFavoriteGame).not.toHaveBeenCalled();
    expect(get(favoriteIds)).toEqual(new Set());
  });

  it("does not lose a favorite when two different games are toggled concurrently", async () => {
    // Model the backend as the authoritative accumulator the real command is.
    const backend: string[] = [];
    addFavoriteGame.mockImplementation(async (id: string) => {
      if (!backend.includes(id)) backend.push(id);
      return [...backend];
    });
    await Promise.all([toggleFavorite("g1"), toggleFavorite("g2")]);
    expect(addFavoriteGame).toHaveBeenCalledTimes(2);
    // The old whole-settings round-trip lost one of these; the atomic path keeps both.
    expect(new Set(get(favoriteIds))).toEqual(new Set(["g1", "g2"]));
  });
});
