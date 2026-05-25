import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import ApplyProgressModal from "@/components/ApplyProgressModal.svelte";
import { activeApplies, downloadProgressByGroup, type ApplyTracker } from "@/lib/stores";

function tracker(over: Partial<ApplyTracker>): ApplyTracker {
  return {
    apply_id: "ap-1",
    group_id: "grp-1",
    game_id: "cyberpunk",
    game_label: "Steam - Cyberpunk 2077",
    dll_path: "C:\\Games\\Cyberpunk 2077\\nvngx_dlss.dll",
    family: "dlss_sr",
    target_version: "2.0.0.0",
    stage: "complete",
    failed_at_stage: null,
    message: "done",
    progress: 100,
    error: null,
    error_class: null,
    attempt: 1,
    bytes_downloaded: 100,
    bytes_total: 100,
    bytes_per_sec: 0,
    started_at: Date.now() - 5000,
    ended_at: Date.now(),
    ...over,
  };
}

async function settle(): Promise<void> {
  await tick();
  await Promise.resolve();
  await Promise.resolve();
  await tick();
}

beforeEach(() => {
  activeApplies.set({});
  downloadProgressByGroup.set({});
});
afterEach(() => {
  activeApplies.set({});
  downloadProgressByGroup.set({});
});

describe("ApplyProgressModal (rendered)", () => {
  it("mounts and renders the dialog shell even with no active applies", async () => {
    const { container } = render(ApplyProgressModal, { props: { onClose: vi.fn() } });
    await settle();
    expect(container.querySelector('[role="dialog"]')).not.toBeNull();
    expect(container.querySelector(".progress-track")).not.toBeNull();
  });

  it("renders a completed group with feature title, version and Updated status", async () => {
    activeApplies.set({ "ap-1": tracker({}) });
    const { container } = render(ApplyProgressModal, { props: { onClose: vi.fn() } });
    await settle();
    expect(container.textContent).toContain("DLSS Super Resolution");
    expect(container.textContent).toContain("v2.0.0.0");
    expect(container.querySelector(".status-pill.is-success")).not.toBeNull();
    expect(container.textContent).toContain("Updated");
  });

  it("exposes the per-group detail toggle and group stat chips in the pane head", async () => {
    activeApplies.set({ "ap-1": tracker({}) });
    const { container } = render(ApplyProgressModal, { props: { onClose: vi.fn() } });
    await settle();
    expect(container.querySelector(".detail-toggle")).not.toBeNull();
    expect(container.querySelector(".pane-head-stats")).not.toBeNull();
    expect(container.querySelector(".phs-progress")).not.toBeNull();
  });

  it("shows the Dismiss aura-pill in the footer once all applies are terminal", async () => {
    activeApplies.set({ "ap-1": tracker({}) });
    const { container } = render(ApplyProgressModal, { props: { onClose: vi.fn() } });
    await settle();
    const footerPills = Array.from(container.querySelectorAll(".action-cta .aura-pill")).map((b) => b.textContent?.trim());
    expect(footerPills.join(" ")).toContain("Dismiss");
  });

  it("the detail toggle flips between hide/show and gates the file-stage detail", async () => {
    activeApplies.set({ "ap-1": tracker({ stage: "downloading", ended_at: null, progress: 40 }) });
    const { container } = render(ApplyProgressModal, { props: { onClose: vi.fn() } });
    await settle();
    const toggle = container.querySelector(".detail-toggle") as HTMLButtonElement;
    expect(toggle.textContent?.trim()).toBe("Hide detail");
    expect(container.querySelector(".file-stage")).not.toBeNull();
    toggle.click();
    await settle();
    expect(toggle.textContent?.trim()).toBe("Show detail");
  });
});
