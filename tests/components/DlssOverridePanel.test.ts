import { describe, it, expect } from "vitest";
import { render } from "@testing-library/svelte";
import DlssOverridePanel from "@/components/DlssOverridePanel.svelte";

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
});
