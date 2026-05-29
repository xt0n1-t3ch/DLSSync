import { describe, it, expect, beforeEach } from "vitest";
import { tick } from "svelte";
import { get } from "svelte/store";
import { render, fireEvent } from "@testing-library/svelte";
import ActivityDock from "@/components/ActivityDock.svelte";
import {
  activeApplies,
  driverInstall,
  systemDriverInstall,
  applyModalOpen,
  type ApplyTracker,
} from "@/lib/stores";

function tracker(over: Partial<ApplyTracker> = {}): ApplyTracker {
  return {
    apply_id: "a1",
    group_id: "g1",
    game_id: "cyberpunk",
    game_label: "Cyberpunk 2077",
    dll_path: "nvngx_dlss.dll",
    family: "dlss_sr",
    target_version: "310.2.1",
    stage: "downloading",
    failed_at_stage: null,
    message: "",
    progress: 0.5,
    error: null,
    error_class: null,
    attempt: 1,
    bytes_downloaded: 5,
    bytes_total: 10,
    bytes_per_sec: 1,
    started_at: 0,
    ended_at: null,
    ...over,
  };
}

beforeEach(() => {
  activeApplies.set({});
  driverInstall.set({ vendor: null, stage: null, message: "", fraction: null });
  systemDriverInstall.set({ updateId: null, stage: null, message: "", fraction: null });
  applyModalOpen.set(false);
});

describe("ActivityDock (rendered)", () => {
  it("renders nothing while idle", () => {
    const { container } = render(ActivityDock);
    expect(container.querySelector(".activity-dock")).toBeNull();
  });

  it("shows a single active apply with its label and a determinate progress fill", async () => {
    const { container } = render(ActivityDock);
    activeApplies.set({ a1: tracker({ progress: 0.5 }) });
    await tick();
    const dock = container.querySelector(".activity-dock");
    expect(dock).not.toBeNull();
    expect(container.querySelector(".dock-headline")?.textContent).toBe("Cyberpunk 2077");
    const fill = container.querySelector(".dock-fill") as HTMLElement | null;
    expect(fill?.style.width).toBe("50%");
  });

  it("summarizes multiple concurrent tasks (apply + driver) as a count", async () => {
    const { container } = render(ActivityDock);
    activeApplies.set({ a1: tracker(), a2: tracker({ apply_id: "a2", game_label: "inZOI" }) });
    driverInstall.set({ vendor: "nvidia", stage: "installing", message: "", fraction: null });
    await tick();
    expect(container.querySelector(".dock-headline")?.textContent).toBe("3 tasks running");
  });

  it("uses an indeterminate fill when no fraction is known", async () => {
    const { container } = render(ActivityDock);
    driverInstall.set({ vendor: "intel", stage: "installing", message: "", fraction: null });
    await tick();
    expect(container.querySelector(".dock-fill-indeterminate")).not.toBeNull();
  });

  it("opens the apply modal when expanded and an apply is active", async () => {
    const { container } = render(ActivityDock);
    activeApplies.set({ a1: tracker() });
    await tick();
    await fireEvent.click(container.querySelector(".dock-expand")!);
    expect(get(applyModalOpen)).toBe(true);
  });

  it("ignores ended applies (only live work shows)", async () => {
    const { container } = render(ActivityDock);
    activeApplies.set({ a1: tracker({ ended_at: 123 }) });
    await tick();
    expect(container.querySelector(".activity-dock")).toBeNull();
  });
});
