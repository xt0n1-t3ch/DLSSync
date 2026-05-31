import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
const api = readFileSync(resolve(root, "lib/api.ts"), "utf8");
const controller = readFileSync(resolve(root, "lib/applyController.ts"), "utf8");
const drawer = readFileSync(resolve(root, "components/GameDetailDrawer.svelte"), "utf8");

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
    expect(controller).toMatch(/rolled back to the previous set/);
  });
});

describe("GameDetailDrawer — Update Streamline set action", () => {
  it("derives the outdated, non-enabler-managed sl.* set members", () => {
    expect(drawer).toMatch(/streamlineSetMembers\s*=\s*\$derived/);
    expect(drawer).toMatch(/isStreamlinePlugin\(filenameFromPath\(r\.path\)\)/);
    expect(drawer).toMatch(/&& isOutdated\(r\) && !enablerManagedSl\(r\)/);
  });

  it("renders the set action gated on members and routes through dispatchStreamlineSet", () => {
    expect(drawer).toMatch(/\{#if streamlineSetMembers\.length > 0\}/);
    expect(drawer).toContain("Update Streamline set");
    expect(drawer).toMatch(/dispatchStreamlineSet\(targets/);
  });
});
