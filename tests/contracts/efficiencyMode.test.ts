import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const controller = readFileSync(resolve(__dirname, "../../frontend/src/components/EfficiencyModeController.svelte"), "utf8");
const toggles = readFileSync(resolve(__dirname, "../../frontend/src/components/PerformanceToggles.svelte"), "utf8");
const en = readFileSync(resolve(__dirname, "../../frontend/src/lib/i18n/locales/en.json"), "utf8");
const es = readFileSync(resolve(__dirname, "../../frontend/src/lib/i18n/locales/es.json"), "utf8");

describe("Efficiency Mode contract", () => {
  it("applies EcoQoS immediately and reasserts it while enabled", () => {
    expect(controller).toContain("setEfficiencyMode(enable)");
    expect(controller).toContain("await applyEfficiency(true, { force: true })");
    expect(controller).toContain("setInterval");
    expect(controller).toContain("EFFICIENCY_REASSERT_MS");
  });

  it("is not tied to focus or visibility anymore", () => {
    expect(controller + toggles).not.toContain("visibilitychange");
    expect(controller + toggles).not.toContain("onFocusChanged");
    expect(controller + toggles).not.toContain("document.visibilityState");
  });

  it("describes always-on Task Manager behavior instead of minimize-only behavior", () => {
    expect(en).toContain("Efficiency Mode always on");
    expect(es).toContain("Modo de eficiencia siempre activo");
    expect(en).not.toContain("Efficiency Mode when minimized");
    expect(es).not.toContain("Modo de eficiencia al minimizar");
  });
});
