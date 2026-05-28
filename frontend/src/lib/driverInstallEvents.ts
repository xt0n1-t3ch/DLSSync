import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { DRIVER_INSTALL_EVENT, type DriverInstallProgress } from "./api";
import { applyDriverInstallProgress } from "./stores";

let unlisten: UnlistenFn | null = null;

/** Register the single app-level listener for driver-install progress. Mounted
 *  once in `App.svelte` so progress keeps flowing into the shared store even
 *  while the Drivers view is unmounted (e.g. the user switched tabs). */
export async function installDriverInstallListener(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen<DriverInstallProgress>(DRIVER_INSTALL_EVENT, (event) => {
    applyDriverInstallProgress(event.payload);
  });
}

export async function uninstallDriverInstallListener(): Promise<void> {
  if (!unlisten) return;
  try {
    unlisten();
  } catch {
  }
  unlisten = null;
}
