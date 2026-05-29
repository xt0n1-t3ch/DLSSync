import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  DRIVER_INSTALL_EVENT,
  SYSTEM_DRIVER_INSTALL_EVENT,
  type DriverInstallProgress,
  type SystemDriverProgress,
} from "./api";
import { applyDriverInstallProgress, applySystemDriverProgress } from "./stores";

let unlisten: UnlistenFn | null = null;
let unlistenSystem: UnlistenFn | null = null;

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

/** App-level listener for general PC-driver (System & Components) install
 *  progress. Same lifetime rationale as the GPU listener above. */
export async function installSystemDriverListener(): Promise<void> {
  if (unlistenSystem) return;
  unlistenSystem = await listen<SystemDriverProgress>(SYSTEM_DRIVER_INSTALL_EVENT, (event) => {
    applySystemDriverProgress(event.payload);
  });
}

export async function uninstallSystemDriverListener(): Promise<void> {
  if (!unlistenSystem) return;
  try {
    unlistenSystem();
  } catch {
  }
  unlistenSystem = null;
}
