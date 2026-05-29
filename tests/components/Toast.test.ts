import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import Toast from "@/components/Toast.svelte";
import { showToast, toasts } from "@/lib/stores";

beforeEach(() => {
  vi.useFakeTimers();
  toasts.set([]);
});
afterEach(() => {
  toasts.set([]);
  vi.clearAllTimers();
  vi.useRealTimers();
});

describe("Toast popup (rendered)", () => {
  it("renders no toast nodes when the store is empty", () => {
    const { container } = render(Toast);
    expect(container.querySelectorAll(".toast").length).toBe(0);
  });

  it("renders a toast with its message and kind class", async () => {
    const { container } = render(Toast);
    showToast("success", "Updated Cyberpunk 2077");
    await tick();
    expect(container.querySelector(".toast.toast-success")).not.toBeNull();
    expect(container.textContent).toContain("Updated Cyberpunk 2077");
  });

  it("stacks multiple toasts and tags each kind", async () => {
    const { container } = render(Toast);
    showToast("info", "one");
    showToast("danger", "two");
    await tick();
    expect(container.querySelectorAll(".toast").length).toBe(2);
    expect(container.querySelector(".toast-danger")).not.toBeNull();
  });

  it("exposes a dismiss button per toast", async () => {
    const { container } = render(Toast);
    showToast("warning", "closable");
    await tick();
    expect(container.querySelector(".toast-close")).not.toBeNull();
  });

  it("renders a kind-icon tinted to the toast kind", async () => {
    const { container } = render(Toast);
    showToast("danger", "boom");
    await tick();
    const icon = container.querySelector(".toast-icon");
    expect(icon).not.toBeNull();
    expect(icon?.getAttribute("data-kind")).toBe("danger");
    expect(icon?.querySelector("svg")).not.toBeNull();
  });

  it("renders an auto-dismiss progress indicator carrying the ttl duration", async () => {
    const { container } = render(Toast);
    showToast("info", "draining", 5000);
    await tick();
    const progress = container.querySelector(".toast-progress") as HTMLElement | null;
    expect(progress).not.toBeNull();
    expect(progress?.getAttribute("data-kind")).toBe("info");
    expect(progress?.style.getPropertyValue("--toast-ttl")).toBe("5000ms");
  });
});
