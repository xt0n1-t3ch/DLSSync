import { describe, expect, it } from "vitest";
import {
  MOUSE_BACK_BUTTON,
  MOUSE_FORWARD_BUTTON,
  createAppNavigationHistory,
  navigationDirectionForMouseButton,
} from "@/lib/appNavigation";

describe("app navigation history", () => {
  it("moves backward and forward through view states", () => {
    const history = createAppNavigationHistory({ view: "library", drawerGameId: null });

    history.record({ view: "catalog", drawerGameId: null });
    history.record({ view: "drivers", drawerGameId: null });

    expect(history.canMove("back")).toBe(true);
    expect(history.move("back")).toEqual({ view: "catalog", drawerGameId: null });
    expect(history.move("back")).toEqual({ view: "library", drawerGameId: null });
    expect(history.move("back")).toBeNull();
    expect(history.canMove("forward")).toBe(true);
    expect(history.move("forward")).toEqual({ view: "catalog", drawerGameId: null });
  });

  it("tracks detail rail open and close as navigable states", () => {
    const history = createAppNavigationHistory({ view: "library", drawerGameId: null });

    history.record({ view: "library", drawerGameId: "game-1" });
    history.record({ view: "library", drawerGameId: null });

    expect(history.move("back")).toEqual({ view: "library", drawerGameId: "game-1" });
    expect(history.move("back")).toEqual({ view: "library", drawerGameId: null });
  });

  it("does not duplicate unchanged states", () => {
    const history = createAppNavigationHistory({ view: "library", drawerGameId: null });

    history.record({ view: "library", drawerGameId: null });

    expect(history.entries).toHaveLength(1);
  });

  it("drops forward history after recording a new branch", () => {
    const history = createAppNavigationHistory({ view: "library", drawerGameId: null });

    history.record({ view: "catalog", drawerGameId: null });
    history.record({ view: "drivers", drawerGameId: null });
    expect(history.move("back")).toEqual({ view: "catalog", drawerGameId: null });

    history.record({ view: "settings", drawerGameId: null });

    expect(history.entries.map((entry) => entry.view)).toEqual(["library", "catalog", "settings"]);
    expect(history.canMove("forward")).toBe(false);
  });
});

describe("navigationDirectionForMouseButton", () => {
  it("maps side mouse buttons to app history directions", () => {
    expect(navigationDirectionForMouseButton(MOUSE_BACK_BUTTON)).toBe("back");
    expect(navigationDirectionForMouseButton(MOUSE_FORWARD_BUTTON)).toBe("forward");
  });

  it("ignores primary, middle, and secondary mouse buttons", () => {
    expect(navigationDirectionForMouseButton(0)).toBeNull();
    expect(navigationDirectionForMouseButton(1)).toBeNull();
    expect(navigationDirectionForMouseButton(2)).toBeNull();
  });
});
