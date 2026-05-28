import { describe, it, expect, vi, beforeEach } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import DriverHistoryFlyout from "@/components/DriverHistoryFlyout.svelte";
import { driverHistory, driverHistoryLoading } from "@/lib/stores";
import type { DriverReleaseDto } from "@/lib/api";

vi.mock("@/lib/stores", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/stores")>();
  return {
    ...actual,
    loadDriverHistory: vi.fn(async () => undefined),
  };
});

function release(version: string, packed: number, isBeta: boolean): DriverReleaseDto {
  return {
    vendor: "nvidia",
    version: { display: version, packed, raw: `${packed}` },
    channel: isBeta ? "beta" : "stable",
    display_version: null,
    is_beta: isBeta,
    download_url: `https://example.test/${version}.exe`,
    size_bytes: 800 * 1024 * 1024,
    signature_subject: "NVIDIA Corporation",
    released_at: "2026-05-26T00:00:00Z",
    release_notes_url: null,
    changelog: null,
  } as DriverReleaseDto;
}

const MODEL = "GeForce RTX 4070 Ti SUPER";

const baseProps = {
  vendor: "nvidia" as const,
  model: MODEL,
  accent: "#76b900",
  onClose: () => undefined,
};

beforeEach(() => {
  driverHistory.set({});
  driverHistoryLoading.set({});
});

describe("DriverHistoryFlyout WHQL toggle honesty", () => {
  it("disables the toggle and switches the label when no betas are loaded", async () => {
    const releases = Array.from({ length: 50 }, (_, i) =>
      release(`610.${50 - i}`, 61050 - i, false),
    );
    driverHistory.set({ [MODEL]: releases });

    const { container } = render(DriverHistoryFlyout, { props: baseProps });
    await tick();

    const toggle = container.querySelector('[role="checkbox"]') as HTMLButtonElement | null;
    expect(toggle).not.toBeNull();
    expect(toggle?.hasAttribute("disabled")).toBe(true);
    expect(toggle?.textContent ?? "").toContain("all loaded drivers WHQL");
    expect(container.textContent ?? "").toContain("v610.50");
  });

  it("keeps the toggle enabled when betas are present and shows the hidden count once checked", async () => {
    const whql = Array.from({ length: 47 }, (_, i) => release(`610.${47 - i}`, 61047 - i, false));
    const beta = Array.from({ length: 3 }, (_, i) => release(`611.${3 - i}b`, 61103 - i, true));
    driverHistory.set({ [MODEL]: [...beta, ...whql] });

    const { container, getByRole } = render(DriverHistoryFlyout, { props: baseProps });
    await tick();

    const toggle = getByRole("checkbox") as HTMLButtonElement;
    expect(toggle.hasAttribute("disabled")).toBe(false);
    expect(toggle.textContent ?? "").toContain("WHQL only");

    toggle.click();
    await tick();

    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(toggle.textContent ?? "").toContain("(-3)");
    const footer = container.querySelector(".flyout-foot")?.textContent ?? "";
    expect(footer).toMatch(/47 of 50/);
  });
});
