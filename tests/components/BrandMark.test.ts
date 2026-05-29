import { afterEach, describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/svelte";
import BrandMark from "@/components/BrandMark.svelte";
import { BRANDS } from "@/lib/brands";

afterEach(() => {
  vi.unstubAllEnvs();
});

describe("BrandMark — bundled official marks", () => {
  it("renders an inline svg with the brand path for a known key", () => {
    const { container } = render(BrandMark, { props: { key: "nvidia" } });
    const svg = container.querySelector("svg.brand-glyph");
    expect(svg).not.toBeNull();
    const path = svg?.querySelector("path");
    expect(path?.getAttribute("d")).toBe(BRANDS.nvidia.path);
    expect(container.textContent).toContain("NVIDIA");
  });

  it("resolves a messy provider string to the right glyph and clean label", () => {
    const { container } = render(BrandMark, { props: { key: "Advanced Micro Devices, Inc." } });
    const path = container.querySelector("svg.brand-glyph path");
    expect(path?.getAttribute("d")).toBe(BRANDS.amd.path);
    expect(container.textContent?.trim()).toBe("AMD");
  });

  it("hides the label when showLabel is false but keeps the glyph", () => {
    const { container } = render(BrandMark, { props: { key: "intel", showLabel: false } });
    expect(container.querySelector("svg.brand-glyph")).not.toBeNull();
    expect(container.querySelector(".brand-label")).toBeNull();
  });

  it("tints the glyph with the vendor token in color tone", () => {
    const { container } = render(BrandMark, { props: { key: "nvidia", tone: "color" } });
    const svg = container.querySelector("svg.brand-glyph") as SVGElement | null;
    expect(svg?.getAttribute("style") ?? "").toContain("var(--vendor-nvidia)");
  });

  it("uses currentColor in mono tone so it inherits the host color", () => {
    const { container } = render(BrandMark, { props: { key: "nvidia", tone: "mono" } });
    const svg = container.querySelector("svg.brand-glyph") as SVGElement | null;
    expect((svg?.getAttribute("style") ?? "").toLowerCase()).toContain("currentcolor");
  });

  it("prefers an explicit label over the resolved brand label", () => {
    const { container } = render(BrandMark, { props: { key: "amd", label: "AMD Radeon" } });
    expect(container.textContent?.trim()).toBe("AMD Radeon");
    expect(container.querySelector("svg.brand-glyph path")?.getAttribute("d")).toBe(BRANDS.amd.path);
  });
});

describe("BrandMark — dynamic Brandfetch fallback", () => {
  it("renders a brandfetch <img> for a domain-resolved provider when a client id is configured", () => {
    vi.stubEnv("VITE_BRANDFETCH_CLIENT_ID", "test123");
    const { container } = render(BrandMark, { props: { key: "Realtek Semiconductor Corp." } });
    expect(container.querySelector("svg.brand-glyph")).toBeNull();
    const img = container.querySelector("img.brand-img") as HTMLImageElement | null;
    expect(img).not.toBeNull();
    expect(img?.getAttribute("src")).toContain("cdn.brandfetch.io/realtek.com");
    expect(img?.getAttribute("src")).toContain("c=test123");
    expect(container.textContent).toContain("Realtek");
  });

  it("falls back to label-only (no img, no glyph) for a domain-resolved provider with no client id", () => {
    vi.stubEnv("VITE_BRANDFETCH_CLIENT_ID", "");
    const { container } = render(BrandMark, { props: { key: "Realtek Semiconductor Corp." } });
    expect(container.querySelector("svg.brand-glyph")).toBeNull();
    expect(container.querySelector("img.brand-img")).toBeNull();
    expect(container.textContent?.trim()).toBe("Realtek Semiconductor Corp.");
  });

  it("falls back to label-only with no empty glyph for an unknown provider", () => {
    vi.stubEnv("VITE_BRANDFETCH_CLIENT_ID", "test123");
    const { container } = render(BrandMark, { props: { key: "Acme Widgets LLC" } });
    expect(container.querySelector("svg.brand-glyph")).toBeNull();
    expect(container.querySelector("img.brand-img")).toBeNull();
    expect(container.textContent?.trim()).toBe("Acme Widgets LLC");
  });

  it("renders nothing visible when provider is unknown and showLabel is false", () => {
    const { container } = render(BrandMark, {
      props: { key: "Acme Widgets LLC", showLabel: false },
    });
    expect(container.querySelector("svg.brand-glyph")).toBeNull();
    expect(container.querySelector("img.brand-img")).toBeNull();
    expect(container.querySelector(".brand-label")).toBeNull();
  });
});
