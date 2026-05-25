import { describe, it, expect } from "vitest";
import { makeNotificationEntry, type NotificationKind } from "@/lib/notifications";

const ALL_KINDS: NotificationKind[] = [
  "apply_success",
  "apply_failure",
  "apply_cancelled",
  "app_update_available",
  "catalog_update_available",
  "scan_failed",
  "catalog_refresh_failed",
];

describe("makeNotificationEntry", () => {
  it("builds a complete unread, undismissed entry", () => {
    const e = makeNotificationEntry("apply_success", "Updated Cyberpunk 2077");
    expect(e.kind).toBe("apply_success");
    expect(e.title).toBe("Updated Cyberpunk 2077");
    expect(e.body).toBeNull();
    expect(e.read_at).toBeNull();
    expect(e.dismissed_at).toBeNull();
    expect(e.apply_id).toBeNull();
    expect(e.game_id).toBeNull();
    expect(e.error_class).toBeNull();
    expect(typeof e.id).toBe("string");
    expect(e.id.length).toBeGreaterThan(0);
    expect(() => new Date(e.created_at).toISOString()).not.toThrow();
  });

  it("carries an optional body and extras", () => {
    const e = makeNotificationEntry("apply_failure", "Apply failed", "lock: file in use", {
      apply_id: "ap-1",
      game_id: "g-1",
      error_class: "lock",
    });
    expect(e.body).toBe("lock: file in use");
    expect(e.apply_id).toBe("ap-1");
    expect(e.game_id).toBe("g-1");
    expect(e.error_class).toBe("lock");
  });

  it("generates a unique id per call", () => {
    const a = makeNotificationEntry("scan_failed", "x");
    const b = makeNotificationEntry("scan_failed", "x");
    expect(a.id).not.toBe(b.id);
  });

  it("created_at is ISO-8601 and recent", () => {
    const e = makeNotificationEntry("catalog_update_available", "DLSS 2.0 available");
    const t = new Date(e.created_at).getTime();
    expect(Math.abs(Date.now() - t)).toBeLessThan(5000);
  });

  it("supports every NotificationKind", () => {
    for (const k of ALL_KINDS) {
      expect(makeNotificationEntry(k, `t-${k}`).kind).toBe(k);
    }
  });
});
