import type { Action } from "svelte/action";

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusable(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
}

export const focusTrap: Action<HTMLElement> = (node) => {
  const opener = document.activeElement as HTMLElement | null;

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Tab") return;
    const items = focusable(node);
    if (items.length === 0) {
      event.preventDefault();
      node.focus();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (event.shiftKey && (active === first || !node.contains(active))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  const initial = focusable(node)[0] ?? node;
  if (!node.contains(document.activeElement)) {
    if (initial === node && node.tabIndex < 0) node.tabIndex = -1;
    initial.focus();
  }
  node.addEventListener("keydown", onKeydown);

  return {
    destroy() {
      node.removeEventListener("keydown", onKeydown);
      opener?.focus?.();
    },
  };
};

export default focusTrap;
