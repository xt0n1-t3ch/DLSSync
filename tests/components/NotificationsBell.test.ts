import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { render } from "@testing-library/svelte";
import NotificationsBell from "@/components/NotificationsBell.svelte";
import { notifications, makeNotificationEntry } from "@/lib/notifications";

const here = dirname(fileURLToPath(import.meta.url));
const readSrc = (rel: string): string =>
  readFileSync(resolve(here, "../../frontend/src", rel), "utf8");

beforeEach(() => notifications.set([]));
afterEach(() => notifications.set([]));

describe("NotificationsBell popup (rendered)", () => {
  it("renders nothing when closed", () => {
    const { container } = render(NotificationsBell, { props: { open: false, onClose: vi.fn() } });
    expect(container.querySelector(".bell-panel")).toBeNull();
  });

  it("shows the empty state when open with no entries", async () => {
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelector(".bell-panel")).not.toBeNull();
    expect(container.textContent).toContain("All caught up.");
  });

  it("lists seeded notifications with titles, count and a dismiss control", async () => {
    notifications.set([
      makeNotificationEntry("apply_success", "Updated Cyberpunk 2077"),
      makeNotificationEntry("scan_failed", "Library scan failed", "registry read error"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelectorAll(".bell-item").length).toBe(2);
    expect(container.textContent).toContain("Updated Cyberpunk 2077");
    expect(container.textContent).toContain("Library scan failed");
    expect(container.querySelector(".bell-panel-count")?.textContent).toBe("2");
    expect(container.querySelector(".bell-item-dismiss")).not.toBeNull();
    expect(container.querySelector(".bell-panel-action")?.textContent).toContain("Mark all read");
  });

  it("maps each kind to its badge tint", async () => {
    notifications.set([
      makeNotificationEntry("apply_success", "ok"),
      makeNotificationEntry("apply_failure", "bad"),
      makeNotificationEntry("app_update_available", "v2"),
      makeNotificationEntry("catalog_update_available", "new catalog release"),
      makeNotificationEntry("catalog_refresh_failed", "warn"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    const tints = Array.from(container.querySelectorAll(".bell-item-badge")).map((b) => b.getAttribute("data-tint"));
    expect(tints).toEqual(["green", "red", "blue", "purple", "orange"]);
  });

  it("renders a vendor brand logo (not a tint badge) for tech/vendor notifications", async () => {
    notifications.set([
      makeNotificationEntry("catalog_update_available", "DLSS 310.6 available"),
      makeNotificationEntry("driver_update_available", "GPU driver 999 — RTX 4070 Ti SUPER"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelectorAll(".bell-item-logo").length).toBe(2);
    expect(container.querySelectorAll(".bell-item-badge[data-tint]").length).toBe(0);
  });

  it("renders external link actions: release entry gets GitHub + Nexus, driver entry gets one", async () => {
    notifications.set([
      makeNotificationEntry("app_update_available", "DLSSync v1.6.0 available", "Fixed XeSS", {
        link: "https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.0",
      }),
      makeNotificationEntry("driver_update_available", "GPU driver 999.99 — RTX 4070 Ti SUPER", "New NVIDIA driver", {
        link: "https://www.nvidia.com/notes",
      }),
      makeNotificationEntry("apply_success", "no links here"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    const items = container.querySelectorAll(".bell-item");
    const releaseLinks = items[0].querySelectorAll(".bell-item-link");
    expect(releaseLinks.length).toBe(2);
    expect(Array.from(releaseLinks).map((b) => b.textContent?.trim())).toEqual(
      expect.arrayContaining([expect.stringContaining("GitHub release"), expect.stringContaining("Nexus Mods")]),
    );
    expect(items[1].querySelectorAll(".bell-item-link").length).toBe(1);
    expect(items[2].querySelectorAll(".bell-item-link").length).toBe(0);
  });

  it("maps the new kinds to badge tints", async () => {
    notifications.set([
      makeNotificationEntry("driver_update_available", "gpu"),
      makeNotificationEntry("system_driver_update_available", "sys"),
      makeNotificationEntry("backup_restored", "restored"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    const tints = Array.from(container.querySelectorAll(".bell-item-badge")).map((b) => b.getAttribute("data-tint"));
    expect(tints).toEqual(["green", "purple", "green"]);
  });

  it("renders the dll-updates digest with a blue tint badge (no vendor logo)", async () => {
    notifications.set([
      makeNotificationEntry("dll_updates_available", "8 updates ready in 3 games", "Open the Library to apply them"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelectorAll(".bell-item-logo").length).toBe(0);
    const badge = container.querySelector(".bell-item-badge[data-tint]");
    expect(badge?.getAttribute("data-tint")).toBe("blue");
    expect(container.textContent).toContain("8 updates ready in 3 games");
  });

  it("marks unread entries with the accent stripe", async () => {
    notifications.set([makeNotificationEntry("apply_success", "unread one")]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelector(".bell-item-unread")).not.toBeNull();
    expect(container.querySelector(".bell-unread-stripe")).not.toBeNull();
  });
});

describe("NotificationsBell mount-location contract (frosted-glass fix)", () => {
  it("panel is fixed-positioned, not absolute inside the TopBar", () => {
    const src = readSrc("components/NotificationsBell.svelte");
    expect(src).toMatch(/\.bell-panel\s*\{[^}]*position:\s*fixed/);
    expect(src).not.toMatch(/\.bell-panel\s*\{[^}]*position:\s*absolute/);
  });

  it("TopBar no longer nests the panel inside its glass backdrop root", () => {
    const topbar = readSrc("components/TopBar.svelte");
    expect(topbar).not.toContain("NotificationsBell");
    expect(topbar).toContain("data-notifications-toggle");
  });

  it("the panel is mounted at the app root next to the command palette", () => {
    const app = readSrc("App.svelte");
    expect(app).toContain("<NotificationsBell");
    expect(app).toContain("notificationsOpen");
  });
});
