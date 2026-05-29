import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/svelte";
import DlssOverridePanel from "@/components/DlssOverridePanel.svelte";
import { emptyDlssConfig } from "@/lib/dlss";
import * as api from "@/lib/api";

const globalScope = { scope: "global" } as const;

describe("DlssOverridePanel", () => {
  it("renders both feature groups and the reversible / anti-cheat note", () => {
    const { getByText, container } = render(DlssOverridePanel, {
      props: { scope: globalScope, driverPacked: 61047 },
    });
    expect(getByText("Super Resolution")).toBeTruthy();
    expect(getByText("Frame Generation")).toBeTruthy();
    const text = (container.textContent ?? "").replace(/\s+/g, " ");
    expect(text).toContain("Fully reversible");
    expect(text).toContain("anti-cheat may flag");
  });

  it("renders custom dropdowns + checkboxes (no native controls)", () => {
    const { container } = render(DlssOverridePanel, {
      props: { scope: globalScope, driverPacked: 61047 },
    });
    const text = container.textContent ?? "";
    expect(text).toContain("No preset override");
    expect(text).toContain("No mode override");
    expect(container.querySelectorAll(".sel-trigger").length).toBeGreaterThanOrEqual(2);
    expect(container.querySelectorAll('[role="checkbox"]').length).toBe(2);
    expect(container.querySelector("select")).toBeNull();
    expect(container.querySelector('input[type="checkbox"]')).toBeNull();
  });

  it("warns when the driver is too old for DLSS 4", () => {
    const { container } = render(DlssOverridePanel, {
      props: { scope: globalScope, driverPacked: 57000 },
    });
    expect(container.textContent).toContain("572.16 or newer");
  });

  it("hydrates the form from read_dlss_override_config and labels the source (forum #1)", async () => {
    const readback: api.DlssOverrideReadback = {
      config: { ...emptyDlssConfig(), enable_sr_dll_override: true, sr_preset: "k" },
      source: "global",
      active_count: 1,
    };
    const spy = vi.spyOn(api, "readDlssOverrideConfig").mockResolvedValue(readback);
    const { findByText } = render(DlssOverridePanel, {
      props: { scope: globalScope, driverPacked: 61047 },
    });
    expect(await findByText(/Preset K/)).toBeTruthy();
    expect(await findByText("Set in NVIDIA driver")).toBeTruthy();
    spy.mockRestore();
  });
});
