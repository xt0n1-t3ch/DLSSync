import { describe, it, expect } from "vitest";
import schema from "../../contracts/driver-release.schema.json";
import { assertConforms } from "./_schema";

const nvidiaRelease = {
  vendor: "nvidia",
  version: { packed: 61047, display: "610.47", raw: "610.47" },
  channel: "stable",
  is_beta: false,
  display_version: null,
  download_url: "https://us.download.nvidia.com/Windows/610.47/610.47-desktop-win11-dch-whql.exe",
  size_bytes: 1025507328,
  signature_subject: "NVIDIA Corporation",
  released_at: "2026-05-26T00:00:00Z",
  release_notes_url: "https://www.nvidia.com/en-us/drivers/details/271418/",
  changelog: {
    highlights: ["Game Ready for 007 First Light"],
    fixed: ["Enshrouded: Missing terrain after driver update [5955501]"],
    notes_page_url: "https://us.download.nvidia.com/Windows/610.47/610.47-win11-win10-release-notes.pdf",
  },
};

const amdRelease = {
  vendor: "amd",
  version: { packed: 26052, display: "26.5.2", raw: "32.0.31007.5012" },
  channel: "stable",
  is_beta: false,
  display_version: "26.5.2",
  download_url: "https://drivers.amd.com/drivers/whql-amd-software-adrenalin-edition-26.5.2-win11-c.exe",
  size_bytes: 0,
  signature_subject: "Advanced Micro Devices, Inc.",
  released_at: null,
  release_notes_url: "https://www.amd.com/en/resources/support-articles/release-notes/RN-RAD-WIN-26-5-2.html",
  changelog: null,
};

const intelRelease = {
  vendor: "intel",
  version: { packed: 32010108801, display: "32.0.101.8801", raw: "32.0.101.8801" },
  channel: "stable",
  is_beta: false,
  display_version: "32.0.101.8801",
  download_url: "https://downloadmirror.intel.com/919751/gfx_win_101.8801.exe",
  size_bytes: 823031088,
  signature_subject: "Intel Corporation",
  released_at: "2026-05-15T00:00:00Z",
  release_notes_url:
    "https://www.intel.com/content/www/us/en/download/785597/919751/intel-arc-graphics-windows.html",
  changelog: null,
};

describe("driver-release contract", () => {
  it("documents the agreed field set as required", () => {
    expect(schema.required).toEqual([
      "vendor",
      "version",
      "channel",
      "is_beta",
      "download_url",
      "size_bytes",
      "signature_subject",
      "released_at",
      "release_notes_url",
      "display_version",
      "changelog",
    ]);
  });

  it.each([
    ["nvidia", nvidiaRelease],
    ["amd", amdRelease],
    ["intel", intelRelease],
  ])("validates a %s release fixture against the schema", (_vendor, fixture) => {
    assertConforms(fixture, schema as never);
  });
});
