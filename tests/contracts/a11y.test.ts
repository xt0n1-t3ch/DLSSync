import { describe, it, expect } from "vitest";
import { render } from "@testing-library/svelte";
import { tick } from "svelte";
import axe from "axe-core";
import Checkbox from "@/components/Checkbox.svelte";
import CounterPill from "@/components/CounterPill.svelte";
import BrandMark from "@/components/BrandMark.svelte";
import Toast from "@/components/Toast.svelte";
import { showToast, toasts } from "@/lib/stores";

const RUN_CONFIG: axe.RunOptions = {
  runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"] },
  rules: {
    "color-contrast": { enabled: false },
    region: { enabled: false },
    "landmark-one-main": { enabled: false },
    "page-has-heading-one": { enabled: false },
    "meta-viewport": { enabled: false },
    "html-has-lang": { enabled: false },
    "document-title": { enabled: false },
    bypass: { enabled: false },
  },
  resultTypes: ["violations"],
};

async function criticalAndSeriousViolations(container: HTMLElement): Promise<axe.Result[]> {
  const results = await axe.run(container, RUN_CONFIG);
  return results.violations.filter(
    (v) => v.impact === "critical" || v.impact === "serious",
  );
}

function format(violations: axe.Result[]): string {
  if (violations.length === 0) return "(no violations)";
  return violations
    .map(
      (v) =>
        `- ${v.id} [${v.impact}] ${v.help} (${v.nodes.length} node${v.nodes.length === 1 ? "" : "s"})`,
    )
    .join("\n");
}

describe("a11y contract — axe critical/serious sweep on core components", () => {
  it("CounterPill carries no critical/serious WCAG violations", async () => {
    const { container } = render(CounterPill, {
      props: { count: 12, tone: "update", ariaLabel: "12 pending updates" },
    });
    const violations = await criticalAndSeriousViolations(container);
    expect(violations.length, format(violations)).toBe(0);
  });

  it("BrandMark carries no critical/serious WCAG violations", async () => {
    const { container } = render(BrandMark, { props: { key: "nvidia" } });
    const violations = await criticalAndSeriousViolations(container);
    expect(violations.length, format(violations)).toBe(0);
  });

  it("Checkbox carries no critical/serious WCAG violations", async () => {
    const { container } = render(Checkbox, {
      props: { checked: false, label: "Notify me on completion" },
    });
    const violations = await criticalAndSeriousViolations(container);
    expect(violations.length, format(violations)).toBe(0);
  });

  it("Toast (populated) carries no critical/serious WCAG violations", async () => {
    toasts.set([]);
    const { container } = render(Toast);
    showToast("success", "Updated Cyberpunk 2077");
    await tick();
    const violations = await criticalAndSeriousViolations(container);
    expect(violations.length, format(violations)).toBe(0);
    toasts.set([]);
  });
});
