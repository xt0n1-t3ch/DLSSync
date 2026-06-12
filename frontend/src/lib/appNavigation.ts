export const MOUSE_BACK_BUTTON = 3;
export const MOUSE_FORWARD_BUTTON = 4;

export interface AppNavigationState {
  view: string;
  drawerGameId: string | null;
}

export type AppNavigationDirection = "back" | "forward";

export interface AppNavigationHistory {
  readonly entries: readonly AppNavigationState[];
  readonly index: number;
  record(state: AppNavigationState): void;
  move(direction: AppNavigationDirection): AppNavigationState | null;
  canMove(direction: AppNavigationDirection): boolean;
}

function sameState(a: AppNavigationState, b: AppNavigationState): boolean {
  return a.view === b.view && a.drawerGameId === b.drawerGameId;
}

export function createAppNavigationHistory(initial: AppNavigationState): AppNavigationHistory {
  let entries: AppNavigationState[] = [initial];
  let index = 0;
  return {
    get entries() {
      return entries;
    },
    get index() {
      return index;
    },
    record(state) {
      if (sameState(entries[index], state)) return;
      entries = [...entries.slice(0, index + 1), state];
      index = entries.length - 1;
    },
    move(direction) {
      const nextIndex = direction === "back" ? index - 1 : index + 1;
      if (nextIndex < 0 || nextIndex >= entries.length) return null;
      index = nextIndex;
      return entries[index];
    },
    canMove(direction) {
      return direction === "back" ? index > 0 : index < entries.length - 1;
    },
  };
}

export function navigationDirectionForMouseButton(button: number): AppNavigationDirection | null {
  if (button === MOUSE_BACK_BUTTON) return "back";
  if (button === MOUSE_FORWARD_BUTTON) return "forward";
  return null;
}
