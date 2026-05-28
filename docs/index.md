# DLSSync documentation

| Doc | What it covers |
|:---|:---|
| [troubleshooting.md](troubleshooting.md) | Log file location and format, the rolling daily log, the `Report a problem` / `Report issue` flow, log levels, and common failure cases |
| [drivers.md](drivers.md) | The GPU driver updater hub: live per-vendor resolution for NVIDIA / Intel / AMD, version normalization, the "What's new" changelog and Release-notes link per vendor, and the download → verify → launch → track install flow |
| [dlss-overrides.md](dlss-overrides.md) | DLSS preset and frame-generation overrides: where they live (Drivers tab global + game drawer per-game), what each preset/mode means, how NVAPI DRS applies it, and driver requirements |
| [anticheat.md](anticheat.md) | Per-game anti-cheat detection (local binary scan + bundled dataset) and the false-positive ban warning shown before any DLL swap or DLSS override |
| [cdp-validation.md](cdp-validation.md) | Visual validation via CDP: attach to the app's own WebView2 remote-debugging port (NOT Edge, NOT localhost), `Page.captureScreenshot` regardless of foreground, and why the other approaches fail |

Project-level references live at the repo root: [README.md](../README.md),
[CHANGELOG.md](../CHANGELOG.md), and the test-suite index at [tests/index.md](../tests/index.md).
