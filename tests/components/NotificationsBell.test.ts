import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { tick } from "svelte";
import { render } from "@testing-library/svelte";
import NotificationsBell from "@/components/NotificationsBell.svelte";
import { notifications, makeNotificationEntry } from "@/lib/notifications";

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
      makeNotificationEntry("catalog_update_available", "dlss"),
      makeNotificationEntry("catalog_refresh_failed", "warn"),
    ]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    const tints = Array.from(container.querySelectorAll(".bell-item-badge")).map((b) => b.getAttribute("data-tint"));
    expect(tints).toEqual(["green", "red", "blue", "purple", "orange"]);
  });

  it("marks unread entries with the accent stripe", async () => {
    notifications.set([makeNotificationEntry("apply_success", "unread one")]);
    const { container } = render(NotificationsBell, { props: { open: true, onClose: vi.fn() } });
    await tick();
    expect(container.querySelector(".bell-item-unread")).not.toBeNull();
    expect(container.querySelector(".bell-unread-stripe")).not.toBeNull();
  });
});
