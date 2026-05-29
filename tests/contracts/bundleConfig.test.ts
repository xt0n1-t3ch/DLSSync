import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const config = JSON.parse(
  readFileSync(resolve(here, "../../src-tauri/tauri.conf.json"), "utf8"),
);

// dlssync.exe carries CompanyName=xt0n1 / LegalCopyright with Apache-2.0 — so a
describe("bundle hardening contract (no-cert antivirus friction reduction)", () => {
  it("identifies a publisher (becomes the PE CompanyName)", () => {
    expect(config.bundle.publisher).toBe("xt0n1");
  });

  it("carries a homepage and an Apache-2.0 copyright line", () => {
    expect(config.bundle.homepage).toMatch(/^https?:\/\//);
    expect(config.bundle.copyright).toContain("Copyright");
    expect(config.bundle.copyright).toContain("Apache-2.0");
  });

  it("installs per-user with no admin elevation (the critical AV-trust setting)", () => {
    expect(config.bundle.windows.nsis.installMode).toBe("currentUser");
  });

  it("ships an installer icon and lzma compression (not a generic, packer-like blob)", () => {
    expect(config.bundle.windows.nsis.installerIcon).toBeTruthy();
    expect(config.bundle.windows.nsis.compression).toBe("lzma");
  });

  it("uses the WebView2 download bootstrapper (no opaque embedded runtime blob)", () => {
    expect(config.bundle.windows.webviewInstallMode.type).toBe(
      "downloadBootstrapper",
    );
  });

  it("offers nsis + msi targets (portable zip is the lowest-friction channel)", () => {
    expect(config.bundle.targets).toContain("nsis");
    expect(config.bundle.targets).toContain("msi");
  });
});
