import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../frontend/src");
const backups = readFileSync(resolve(root, "views/Backups.svelte"), "utf8");
const drivers = readFileSync(resolve(root, "views/Drivers.svelte"), "utf8");
const api = readFileSync(resolve(root, "lib/api.ts"), "utf8");
const stores = readFileSync(resolve(root, "lib/stores.ts"), "utf8");
const ux = readFileSync(resolve(root, "lib/ux.ts"), "utf8");

describe("Backups — System Drivers section (driver_package)", () => {
  it("splits driver-package backups out of the DLL listing", () => {
    expect(backups).toMatch(/backup_type !== "driver_package"/);
    expect(backups).toMatch(/backup_type === "driver_package"/);
    expect(backups).toMatch(/for \(const b of dllBackups\)/);
  });

  it("renders a filterable System Drivers section grouped by device class", () => {
    expect(backups).toContain("System Drivers");
    expect(backups).toMatch(/driverClassFilter/);
    expect(backups).toMatch(/driverGroups/);
    expect(backups).toMatch(/b\.device_class \?\? "Driver"/);
  });

  it("wires a Roll back action to restoreSystemDriver with a confirm", () => {
    expect(backups).toMatch(/restoreSystemDriver/);
    expect(backups).toMatch(/doDriverRestore/);
    expect(backups).toMatch(/Roll back/);
  });
});

describe("Drivers — admin disclaimer + version history", () => {
  it("shows the centralized admin-elevation note", () => {
    expect(ux).toMatch(/export const ADMIN_ELEVATION_NOTE/);
    expect(drivers).toMatch(/import \{ ADMIN_ELEVATION_NOTE \}/);
    expect(drivers).toContain("{ADMIN_ELEVATION_NOTE}");
  });

  it("lazily loads DriverStore versions per card behind target_inf", () => {
    expect(drivers).toMatch(/toggleVersions/);
    expect(drivers).toMatch(/systemDriverVersions\(update\.target_inf\)/);
    expect(drivers).toMatch(/update\.target_inf/);
    expect(drivers).toContain("Latest available");
  });
});

describe("api/stores — install carries snapshot context", () => {
  it("installSystemDriver forwards a context arg", () => {
    expect(api).toMatch(/installSystemDriver\(\s*updateId: string,\s*context\?: DriverInstallContext,/);
    expect(api).toMatch(/invoke\("install_system_driver", \{ updateId, context: context \?\? null \}\)/);
  });

  it("driverInstallContext maps the matched-device fields", () => {
    expect(api).toMatch(/export function driverInstallContext/);
    expect(api).toMatch(/infName: update\.target_inf/);
    expect(api).toMatch(/hardwareId: update\.target_hardware_id/);
  });

  it("exposes restoreSystemDriver + systemDriverVersions bindings", () => {
    expect(api).toMatch(/restore_system_driver/);
    expect(api).toMatch(/system_driver_versions/);
  });

  it("the install flow builds the context from the update + class label", () => {
    expect(stores).toMatch(/driverInstallContext\(update, deviceClassLabel \?\? update\.class\)/);
  });
});
