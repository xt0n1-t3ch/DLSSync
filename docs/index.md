# DLSSync documentation

| Doc | What it covers |
|:---|:---|
| [troubleshooting.md](troubleshooting.md) | Log file location and format, the rolling daily log, the `Report a problem` / `Report issue` flow, log levels, and common failure cases |
| [drivers.md](drivers.md) | The GPU driver updater hub: live per-vendor resolution for NVIDIA / Intel / AMD, version normalization, the "What's new" changelog and Release-notes link per vendor, and the download → verify → launch → track install flow |
| [dlss-overrides.md](dlss-overrides.md) | DLSS preset and frame-generation overrides: where they live (Drivers tab global + game drawer per-game), what each preset/mode means, how NVAPI DRS applies it, and driver requirements |
| [anticheat.md](anticheat.md) | Per-game anti-cheat detection (local binary scan + bundled dataset) and the false-positive ban warning shown before any DLL swap or DLSS override |
| [cdp-validation.md](cdp-validation.md) | Visual validation via CDP: attach to the app's own WebView2 remote-debugging port (NOT Edge, NOT localhost), `Page.captureScreenshot` regardless of foreground, and why the other approaches fail |
| [translations.md](translations.md) | Contributor + translator guide for the i18n catalogs: where every UI string lives (`locales/<locale>.json`), the `area.component.purpose` key scheme, `{placeholder}` and `_one`/`_other` plural rules, the `_meta.json` sidecar, how to translate or add a language without touching code, the two parity validators, and what a parity failure looks like |
| [release-marketing.md](release-marketing.md) | GitHub/Nexus discovery strategy, current v1.6.9 value proposition, SEO-safe wording, Nexus description order, and the canonical public asset map |
| [nexus-build.md](nexus-build.md) | Nexus Mods distribution lane: manual app updates, stripped updater config/capability, build commands, and moderation wording |

Project-level references live at the repo root: [README.md](../README.md),
[CHANGELOG.md](../CHANGELOG.md), and the test-suite index at [tests/index.md](../tests/index.md).
