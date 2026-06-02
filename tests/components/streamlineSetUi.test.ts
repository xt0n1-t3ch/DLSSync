import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
const api = readFileSync(resolve(root, "lib/api.ts"), "utf8");
const controller = readFileSync(resolve(root, "lib/applyController.ts"), "utf8");
const drawer = readFileSync(resolve(root, "components/GameDetailDrawer.svelte"), "utf8");
const enCatalog = readFileSync(resolve(root, "lib/i18n/locales/en.json"), "utf8");

describe("api — applyStreamlineSet binding", () => {
  it("declares StreamlineSetResult with success/applied/error/rolled_back", () => {
    expect(api).toMatch(/export interface StreamlineSetResult/);
    expect(api).toMatch(/rolled_back: boolean/);
    expect(api).toMatch(/applied: ApplyOutcome\[\]/);
  });

  it("invokes the apply_streamline_set command with items", () => {
    expect(api).toMatch(/export async function applyStreamlineSet/);
    expect(api).toMatch(/invoke\("apply_streamline_set", \{ items \}\)/);
  });
});

describe("applyController — dispatchStreamlineSet", () => {
  it("calls applyStreamlineSet and reuses the tracker/modal plumbing", () => {
    expect(controller).toMatch(/export async function dispatchStreamlineSet/);
    expect(controller).toMatch(/await applyStreamlineSet\(requests\)/);
    expect(controller).toMatch(/prepareApply\(targets\)/);
  });

  it("surfaces rollback in the failure path", () => {
    expect(controller).toMatch(/rolled_back/);
    expect(enCatalog).toMatch(/rolled back to the previous set/);
  });
});

describe("GameDetailDrawer — Update Streamline set action", () => {
  const ux = readFileSync(resolve(root, "lib/ux.ts"), "utf8");

  it("derives the outdated sl.* set members with no enabler-managed exclusion", () => {
    expect(drawer).toMatch(/streamlineSetMembers\s*=\s*\$derived/);
    expect(drawer).toMatch(/isStreamlinePlugin\(filenameFromPath\(r\.path\)\)/);
    expect(drawer).toMatch(/&& isOutdated\(r\)\)/);
    expect(drawer).not.toMatch(/enablerManagedSl/);
  });

  it("renders the set action gated on members and routes through dispatchStreamlineSet", () => {
    expect(drawer).toMatch(/\{#if streamlineSetMembers\.length > 0\}/);
    expect(enCatalog).toContain("Update Streamline set");
    expect(drawer).toMatch(/dispatchStreamlineSet\(targets/);
  });

  it("surfaces that a same-major set update is offered under an enabler (requires Streamline >= 2.11)", () => {
    expect(enCatalog).toMatch(/DLSS Enabler requires NVIDIA Streamline 2\.11 or newer/);
    expect(enCatalog).toMatch(/same major version/);
    expect(drawer).toMatch(/\{#if dlssEnabler\}[\s\S]*?\$t\(["']note\.enablerManaged["']\)/);
    expect(drawer).toMatch(/title=\{`[^`]*\$\{STREAMLINE_OVERRIDE_NOTE\}`\}/);
  });
});
