import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
// The drawer was decomposed (v1.6.7): the apply-time anti-cheat MARKUP/CSS now
// lives in DrawerFooter, while the danger-gate LOGIC stays in the orchestrator
// (GameDetailDrawer). Each assertion reads whichever file owns that concern.
const drawer = readFileSync(resolve(root, "components/GameDetailDrawer.svelte"), "utf8");
const footer = readFileSync(resolve(root, "components/DrawerFooter.svelte"), "utf8");
const enCatalog = JSON.parse(readFileSync(resolve(root, "lib/i18n/locales/en.json"), "utf8"));
const esCatalog = JSON.parse(readFileSync(resolve(root, "lib/i18n/locales/es.json"), "utf8"));

function ruleBody(css: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`);
  return re.exec(css)?.[1] ?? "";
}

describe("GameDetailDrawer — apply-time anti-cheat risk (the differentiator)", () => {
  it("surfaces a stable ac-apply-risk element gated on acActive at the apply footer", () => {
    expect(footer).toMatch(/class="ac-apply-risk"/);
    expect(footer).toMatch(/\{#if acActive && selectedCount > 0\}[\s\S]*?ac-apply-risk/);
  });

  it("colors the apply-time risk chip by severity using --warning / --danger tokens", () => {
    expect(footer).toMatch(/class:is-warning=\{acSeverity !== "danger"\}/);
    expect(footer).toMatch(/class:is-danger=\{acSeverity === "danger"\}/);
    const warn = ruleBody(footer, ".ac-apply-risk.is-warning");
    expect(warn).toMatch(/var\(--warning\)/);
    const danger = ruleBody(footer, ".ac-apply-risk.is-danger");
    expect(danger).toMatch(/var\(--danger\)/);
  });

  it("the apply button dispatches through the danger gate (onRequestApply), not applySelected directly", () => {
    expect(footer).toMatch(/class="[^"]*\bfoot-apply\b[^"]*"/);
    expect(footer).toMatch(/class="[^"]*\bbtn-primary\b[^"]*"/);
    expect(footer).toMatch(/\bfoot-apply\b[\s\S]{0,300}?onclick=\{onRequestApply\}/);
    expect(footer).not.toMatch(/\bfoot-apply\b[\s\S]{0,300}?onclick=\{applySelected\}/);
  });

  it("danger severity requires an explicit confirm before the apply dispatches", () => {
    expect(drawer).toMatch(
      /if \(acActive && acSeverity === "danger" && !acConfirming\) \{\s*acConfirming = true;\s*return;\s*\}/,
    );
    expect(drawer).toMatch(/void applySelected\(\);/);
  });

  it("renders a stable ac-apply-confirm affordance with an accessible alertdialog role", () => {
    expect(footer).toMatch(/\{#if acConfirming\}[\s\S]*?class="ac-apply-confirm"/);
    expect(footer).toMatch(/class="ac-apply-confirm" role="alertdialog"/);
  });

  it("the confirm offers an explicit proceed (apply anyway) and a cancel — never hard-blocks", () => {
    expect(footer).toMatch(/class="[^"]*\bac-confirm-proceed\b[^"]*"[^>]{0,100}onclick=\{onRequestApply\}/);
    expect(footer).toMatch(/class="[^"]*\bac-confirm-cancel\b[^"]*"[^>]{0,100}onclick=\{onCancelApplyConfirm\}/);
    expect(drawer).toMatch(/function cancelApplyConfirm\(\): void \{\s*acConfirming = false;\s*\}/);
  });

  it("warning severity (e.g. Denuvo) only shows the chip and never enters the confirm gate", () => {
    expect(drawer).toMatch(/acSeverity === "danger" && !acConfirming/);
    expect(drawer).not.toMatch(/acSeverity !== "danger" && !acConfirming/);
  });

  it("respects reduced-motion for the confirm reveal", () => {
    expect(footer).toMatch(/@media \(prefers-reduced-motion: reduce\)[\s\S]*?\.ac-apply-confirm \{ animation: none/);
  });

  it("resets the confirm state when switching games", () => {
    expect(drawer).toMatch(/\$effect\(\(\) => \{\s*if \(gameId\) acConfirming = false;\s*\}\);/);
  });
});

describe("anti-cheat apply-risk i18n parity", () => {
  const en = enCatalog.component.gameDrawer.anticheat.apply;
  const es = esCatalog.component.gameDrawer.anticheat.apply;

  it("EN defines every apply-time anti-cheat string", () => {
    for (const k of [
      "chipRisk",
      "chipBan",
      "chipTitle",
      "applyAnyway",
      "confirmAria",
      "confirmBody",
      "confirmBodyGeneric",
      "confirmProceed",
      "confirmCancel",
    ]) {
      expect(typeof en[k], `en ${k}`).toBe("string");
      expect(en[k].length, `en ${k} non-empty`).toBeGreaterThan(0);
    }
  });

  it("ES mirrors the EN keys and keeps the {names} placeholder where EN has it", () => {
    expect(Object.keys(es).sort()).toEqual(Object.keys(en).sort());
    expect(es.confirmBody).toContain("{names}");
    expect(es.chipTitle).toContain("{names}");
  });

  it("the danger confirm copy warns about a ban in both locales", () => {
    expect(en.confirmBody.toLowerCase()).toContain("ban");
    expect(es.confirmBody.toLowerCase()).toContain("baneo");
  });
});
