import { describe, it, expect } from "vitest";
import { tick } from "svelte";
import { render, fireEvent } from "@testing-library/svelte";
import Select from "@/components/Select.svelte";

const opts = [
  { value: null, label: "No override" },
  { value: "k", label: "Preset K" },
  { value: "dyn", label: "Dynamic", disabled: true },
];

describe("Select", () => {
  it("shows the selected option label on the trigger", () => {
    const { getByRole } = render(Select, {
      props: { value: null, options: opts, placeholder: "Pick…" },
    });
    expect(getByRole("button").textContent).toContain("No override");
  });

  it("opens a listbox of options on click", async () => {
    const { getByRole, getAllByRole, queryByRole } = render(Select, {
      props: { value: "k", options: opts },
    });
    expect(queryByRole("listbox")).toBeNull();
    await fireEvent.click(getByRole("button"));
    await tick();
    expect(getByRole("listbox")).toBeTruthy();
    expect(getAllByRole("option").length).toBe(3);
  });

  it("marks the chosen option aria-selected", async () => {
    const { getByRole, getAllByRole } = render(Select, {
      props: { value: "k", options: opts },
    });
    await fireEvent.click(getByRole("button"));
    await tick();
    const chosen = getAllByRole("option").find((o) => o.getAttribute("aria-selected") === "true");
    expect(chosen?.textContent).toContain("Preset K");
  });
});
