import { writable } from "svelte/store";

export const activeArt = writable<string | null>(null);

export function setActiveArt(url: string | null | undefined): void {
  activeArt.set(url && url.length > 0 ? url : null);
}

export function clearActiveArt(): void {
  activeArt.set(null);
}
