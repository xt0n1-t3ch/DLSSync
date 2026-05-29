import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writable, type Writable } from "svelte/store";

export const NOTIFICATION_PUSHED_EVENT = "notification:pushed";

export type NotificationKind =
  | "apply_success"
  | "apply_failure"
  | "apply_cancelled"
  | "app_update_available"
  | "catalog_update_available"
  | "driver_update_available"
  | "system_driver_update_available"
  | "backup_restored"
  | "scan_failed"
  | "catalog_refresh_failed";

export function makeNotificationEntry(
  kind: NotificationKind,
  title: string,
  body: string | null = null,
  extras?: Partial<Pick<NotificationEntry, "apply_id" | "game_id" | "error_class" | "link">>,
): NotificationEntry {
  return {
    id: crypto.randomUUID(),
    kind,
    title,
    body,
    created_at: new Date().toISOString(),
    read_at: null,
    dismissed_at: null,
    apply_id: extras?.apply_id ?? null,
    game_id: extras?.game_id ?? null,
    error_class: extras?.error_class ?? null,
    link: extras?.link ?? null,
  };
}

export interface NotificationEntry {
  id: string;
  kind: NotificationKind;
  title: string;
  body: string | null;
  created_at: string;
  read_at: string | null;
  dismissed_at: string | null;
  apply_id: string | null;
  game_id: string | null;
  error_class: string | null;
  link: string | null;
}

export interface ListFilter {
  include_dismissed?: boolean;
  limit?: number;
}

export const notifications: Writable<NotificationEntry[]> = writable([]);

export async function listNotifications(filter?: ListFilter): Promise<NotificationEntry[]> {
  return invoke("list_notifications", { filter });
}

export async function markRead(id: string): Promise<void> {
  await invoke("mark_notification_read", { id });
  const now = new Date().toISOString();
  notifications.update((arr) =>
    arr.map((e) => (e.id === id && e.read_at == null ? { ...e, read_at: now } : e)),
  );
}

export async function markAllRead(): Promise<number> {
  const count: number = await invoke("mark_all_notifications_read");
  const now = new Date().toISOString();
  notifications.update((arr) => arr.map((e) => (e.read_at ? e : { ...e, read_at: now })));
  return count;
}

export async function dismiss(id: string): Promise<void> {
  await invoke("dismiss_notification", { id });
  notifications.update((arr) => arr.filter((e) => e.id !== id));
}

export async function pushNotification(entry: NotificationEntry): Promise<void> {
  return invoke("push_notification", { entry });
}

export async function unreadCount(): Promise<number> {
  return invoke("notifications_unread_count");
}

export async function refreshNotifications(): Promise<void> {
  try {
    const list = await listNotifications();
    notifications.set(list);
  } catch (err) {
    console.warn("[dlssync] notifications refresh failed:", err);
  }
}

export async function installNotificationsListener(): Promise<UnlistenFn> {
  return listen<NotificationEntry>(NOTIFICATION_PUSHED_EVENT, (event) => {
    const incoming = event.payload;
    notifications.update((arr) => {
      const filtered = arr.filter((e) => e.id !== incoming.id);
      return [incoming, ...filtered];
    });
  });
}
