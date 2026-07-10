import { describe, it, expect } from "vitest";
import { isFavorite, toggleFavorite, filterFavorites } from "@/lib/favorites";

// Pure ownership of UiPreferences.favorite_game_ids. The store action and every
// favorite affordance (card star, list star, Library favorite filter, context
// menu) wire into these helpers, so the behavior is proven here once.
describe("favorites", () => {
  it("isFavorite reflects membership", () => {
    expect(isFavorite(["a", "b"], "a")).toBe(true);
    expect(isFavorite(["a", "b"], "c")).toBe(false);
    expect(isFavorite([], "a")).toBe(false);
  });

  it("toggleFavorite appends an absent id, preserving existing order", () => {
    expect(toggleFavorite(["a"], "b")).toEqual(["a", "b"]);
    expect(toggleFavorite([], "x")).toEqual(["x"]);
  });

  it("toggleFavorite removes a present id without disturbing the rest", () => {
    expect(toggleFavorite(["a", "b", "c"], "b")).toEqual(["a", "c"]);
    expect(toggleFavorite(["only"], "only")).toEqual([]);
  });

  it("toggleFavorite never duplicates and ignores an empty id", () => {
    expect(toggleFavorite(["a"], "a")).toEqual([]);
    expect(toggleFavorite(["a"], "")).toEqual(["a"]);
  });

  it("toggleFavorite does not mutate its input", () => {
    const input = ["a", "b"];
    toggleFavorite(input, "c");
    expect(input).toEqual(["a", "b"]);
  });

  it("filterFavorites keeps only games whose id is favorited, preserving order", () => {
    const games = [{ id: "a" }, { id: "b" }, { id: "c" }];
    expect(filterFavorites(games, new Set(["c", "a"]))).toEqual([{ id: "a" }, { id: "c" }]);
    expect(filterFavorites(games, new Set())).toEqual([]);
  });
});
