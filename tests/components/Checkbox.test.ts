import { describe, it, expect } from "vitest";
import { tick } from "svelte";
import { render, fireEvent } from "@testing-library/svelte";
import Checkbox from "@/components/Checkbox.svelte";

describe("Checkbox", () => {
  it("renders a checkbox role with its label and unchecked state", () => {
    const { getByRole } = render(Checkbox, { props: { checked: false, label: "Force latest DLL" } });
    const cb = getByRole("checkbox");
    expect(cb.getAttribute("aria-checked")).toBe("false");
    expect(cb.textContent).toContain("Force latest DLL");
  });

  it("toggles aria-checked on click", async () => {
    const { getByRole } = render(Checkbox, { props: { checked: false } });
    const cb = getByRole("checkbox");
    await fireEvent.click(cb);
    await tick();
    expect(cb.getAttribute("aria-checked")).toBe("true");
  });

  it("does not toggle when disabled", async () => {
    const { getByRole } = render(Checkbox, { props: { checked: false, disabled: true } });
    const cb = getByRole("checkbox");
    await fireEvent.click(cb);
    await tick();
    expect(cb.getAttribute("aria-checked")).toBe("false");
  });
});
