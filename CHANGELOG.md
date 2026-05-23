# Changelog

All notable changes to DLSSync are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-05-23

First public release.

DLSSync keeps DLSS, FSR, XeSS and DirectStorage DLLs synchronized with NVIDIA, AMD, Intel and Microsoft publisher releases across every game launcher on Windows. Hash-verified, vendor-signed, fully reversible.

### Highlights

- Detects games installed via Steam, Epic, GOG Galaxy, Ubisoft Connect, EA Desktop, Xbox / Microsoft Store and Battle.net out of the box.
- Tracks 18 DLL families across 4 vendors: DLSS SR / FG / RR, Streamline, Reflex, XeSS SR / FG, XeLL, FSR upscaler / FG / loader, DirectStorage.
- Two independent integrity gates per replacement: SHA-256 against the public catalog plus Authenticode publisher subject match. No driver, no kernel hook, no in-process injection.
- Every replaced DLL goes into a local SQLite snapshot store. One-click rollback from the Backups tab, files are also readable directly under `%USERPROFILE%\DLSSync\Backups\`.
- Per-user NSIS install to `%LOCALAPPDATA%\DLSSync\`. No admin prompt, no UAC, no Add or Remove Programs pollution. Portable build runs from anywhere with the same settings layout.

### Added

- Game scanner for Steam, Epic Games, GOG Galaxy, Ubisoft Connect, EA Desktop, Microsoft Store / Xbox and Battle.net, plus arbitrary user-added folders for portable installs.
- Catalog client backed by the [DLSSync-Manifest](https://github.com/xt0n1-t3ch/DLSSync-Manifest) repository, served via jsDelivr with a local on-disk cache and a three-attempt retry ladder.
- Per-DLL version picker exposing every release tracked in the catalog, including historical stable builds and experimental channel entries. Pins survive rescans.
- Tray integration with autostart, minimize-to-tray and Windows EcoQoS Efficiency Mode. Idle CPU drops to about 0 percent and Task Manager shows the green leaf badge.
- In-app auto-update banner. Polls GitHub Releases on a six-hour cadence, verifies the Ed25519 signature over the NSIS bundle, downloads, installs and restarts in place. Portable builds open the release page instead of self-replacing.
- Backups tab with the timestamp, original version, SHA-256 and a Restore button for every snapshot. Snapshots also exist as plain files under `%USERPROFILE%\DLSSync\Backups\`.
- Settings panel: detection probes, advanced flags for development builds, autostart toggle, EcoQoS toggle, manual update check.

### Security

- DLL replacement gated by SHA-256 match against the public catalog AND Authenticode publisher subject match against the NVIDIA, AMD, Intel or Microsoft signing certificate. A mismatch refuses the write.
- Auto-update payload signed with Ed25519. The embedded public key is the only trust anchor, tampered payloads are rejected before extraction.
- Zero telemetry. The only outbound traffic is `api.github.com` for the update check (capped at one request per six hours), `cdn.jsdelivr.net` for the DLL catalog manifest, and Steam's public cover-art CDN for game tiles. Every request is unauthenticated and visible from Settings.

### Footprint

| Metric | Target | Measured |
|---|---|---|
| Installer | under 10 MB | 4.5 MB |
| Cold start | under 500 ms | yes |
| Idle RAM | under 100 MB | yes |
| Idle CPU minimized | about 0 percent | yes (EcoQoS active) |

[1.0.0]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.0.0
