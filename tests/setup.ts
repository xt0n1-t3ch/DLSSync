import { vi } from "vitest";

const tauriInvoke = async (cmd: string): Promise<unknown> => {
  switch (cmd) {
    case "list_notifications":
      return [];
    case "notifications_unread_count":
      return 0;
    case "mark_all_notifications_read":
      return 0;
    case "plugin:event|listen":
      return 0;
    default:
      return undefined;
  }
};

if (typeof globalThis !== "undefined") {
  const internals = {
    invoke: vi.fn(tauriInvoke),
    transformCallback: (cb: unknown) => {
      const id = Math.floor(Math.random() * 1e9);
      (globalThis as Record<string, unknown>)[`_${id}`] = cb;
      return id;
    },
    convertFileSrc: (p: string) => p,
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  };
  const eventInternals = { unregisterListener: () => undefined };
  (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
  (globalThis as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = eventInternals;
  if (typeof window !== "undefined") {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;
    (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = eventInternals;
  }
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    switch (cmd) {
      case "list_notifications":
        return [];
      case "notifications_unread_count":
        return 0;
      case "mark_all_notifications_read":
        return 0;
      default:
        return undefined;
    }
  }),
  convertFileSrc: vi.fn((p: string) => p),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => undefined),
  emit: vi.fn(async () => undefined),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(async () => "0.0.0-test"),
}));

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: vi.fn(async () => null),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  confirm: vi.fn(async () => true),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(async () => undefined),
}));

if (!("randomUUID" in globalThis.crypto)) {
  let counter = 0;
  Object.defineProperty(globalThis.crypto, "randomUUID", {
    value: () => `00000000-0000-4000-8000-${(counter++).toString().padStart(12, "0")}`,
  });
}

if (typeof Element !== "undefined") {
  Element.prototype.animate = function animateStub() {
    const anim: Record<string, unknown> = {
      onfinish: null,
      oncancel: null,
      cancel() {},
      finish() {},
      play() {},
      pause() {},
      reverse() {},
      persist() {},
      updatePlaybackRate() {},
      currentTime: 0,
      startTime: 0,
      playbackRate: 1,
      effect: null,
      playState: "finished",
      finished: Promise.resolve(),
    };
    return anim as unknown as Animation;
  } as typeof Element.prototype.animate;
}

if (typeof window !== "undefined" && !window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    addListener: () => undefined,
    removeListener: () => undefined,
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}
