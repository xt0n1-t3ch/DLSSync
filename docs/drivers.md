# GPU driver updater

The `Drivers` tab keeps the GPU graphics driver current for NVIDIA, Intel and AMD adapters.
It detects each GPU, resolves the latest published driver live from the vendor, and installs the
official vendor package after verifying its signature.

## How detection works

The app reads the installed adapters from DXGI and the installed driver version from WMI
(`Win32_VideoController`), then normalizes each version so the comparison is meaningful:

- NVIDIA: the Windows driver-store version (e.g. `32.0.15.9174`) decodes to the marketing version
  `591.74` from its last five digits.
- Intel: the catalog version is used as-is (e.g. `32.0.101.8801`).
- AMD: the Windows driver-store version is compared against the Adrenalin mapping.

## Where versions come from

Resolution runs live per detected GPU through a `DriverSource` for that vendor:

- NVIDIA — the GeForce driver lookup API (`gfwsl.geforce.com/.../AjaxDriverService.php`), keyed by the
  GPU product id resolved from the `ZenitH-AT/nvidia-data` map and the Windows OS id.
- Intel — the Intel driver catalog (`dsadata.intel.com`).
- AMD — the AMD driver host and the GPUOpen version table.

The `DriverSource` trait and its registry live in the `driver-catalog` crate. GPU is the first
device class; adding another class (chipset, network, audio) means adding a new `DriverSource`
without changing the core.

## Install flow and tracking

1. Download — the official installer streams to the cache directory with byte-accurate progress.
2. Verify — the downloaded `.exe` must carry a valid Authenticode signature from the matching vendor
   (NVIDIA, Intel or AMD); an unsigned or mismatched binary is refused.
3. Launch — the vendor installer runs and self-elevates through the Windows UAC prompt. DLSSync
   itself never requests elevation.
4. Track — the install state machine reports each stage live and resolves the final state from the
   installer exit code (`0`/`3010` succeed, `1602`/`1223` cancel, anything else fails).

## What's new and release notes

Each driver card has a "What's new" expander and a "Release notes" link:

- NVIDIA's lookup returns the release-notes HTML inline, so the card shows the headline ("Game Ready
  for …") and the fixed-issue bullets directly, plus a link to the full notes page.
- Intel and AMD link out to the official release-notes page for the release; the card shows the
  version and date inline.

The "Release notes" link opens the vendor page in the default browser.

## Notes

- A reboot may be required after a successful driver install, per the vendor installer.
- AMD has no reliable per-GPU installer URL, so the AMD card resolves the version and changelog but
  offers "Open driver page" rather than an in-app download.
- If the latest version cannot be resolved for a GPU, the card shows `Unknown` and still offers the
  vendor page where one is known.
