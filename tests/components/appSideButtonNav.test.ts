import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, it, expect } from "vitest";

const APP_SOURCE = resolve(__dirname, "../../frontend/src/App.svelte");
const source = readFileSync(APP_SOURCE, "utf8");
const flat = source.replace(/\s+/g, " ");

describe("App side-button navigation wiring", () => {
  it("uses the appNavigation contract instead of browser history", () => {
    expect(source).toMatch(
      /import\s*\{[^}]*createAppNavigationHistory[^}]*navigationDirectionForMouseButton[^}]*\}\s*from\s*"\.\/lib\/appNavigation"/s,
    );
    expect(flat).toContain("createAppNavigationHistory(");
    expect(source).not.toContain("history.back");
    expect(source).not.toContain("history.forward");
  });

  it("records view and drawer changes as navigable states", () => {
    expect(flat).toMatch(/view:\s*\$currentView,\s*drawerGameId:\s*\$drawerGameId/);
    expect(flat).toContain("navHistory.record(");
  });

  it("maps side mouse buttons through the contract on a window mouseup listener", () => {
    expect(flat).toContain('window.addEventListener("mouseup"');
    expect(flat).toMatch(/navigationDirectionForMouseButton\(\s*e\.button\s*\)/);
  });

  it("prevents default for handled back/forward side-button events", () => {
    expect(flat).toMatch(/if\s*\(\s*!direction\s*\)\s*return;\s*e\.preventDefault\(\)/);
  });

  it("applies a history move only when it can move, then writes both stores", () => {
    expect(flat).toMatch(/navHistory\.canMove\(\s*direction\s*\)/);
    expect(flat).toContain("navHistory.move(");
    expect(flat).toMatch(/currentView\.set\(\s*target\.view\s*\)/);
    expect(flat).toMatch(/drawerGameId\.set\(\s*target\.drawerGameId\s*\)/);
  });

  it("removes the side-button listener on teardown", () => {
    expect(flat).toContain('window.removeEventListener("mouseup"');
  });
});
