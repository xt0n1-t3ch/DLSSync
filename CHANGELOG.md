# Changelog

All notable changes to DLSSync are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-05-24

A top-to-bottom visual rebuild on a more premium design language, a working
notifications channel, and a hardened apply pipeline. Skips 1.1.

### Added — premium UI refresh (light and dark)

- Dual theme: ultra-white (`#FFFFFF`) light and ultra-black (`#000000`) dark, both with floating cards, generous rounded corners, soft shadows, and a refined Apple-blue accent (`#007AFF` light, `#0A84FF` dark) that replaces the previous cyan.
- A small set of shared UI primitives — card, icon badge, and pill button — drives every surface from one token sink, so the whole app restyles consistently instead of bespoke per-component CSS.
- Sidebar rebuilt as an icon rail with a single solid-accent active pill. The old catalog-freshness footer card and the empty vertical gap below the nav are gone.
- Top bar gains a translucent backdrop-blur surface, an icon-only command-palette trigger, and quieter window controls.
- Library hero band, Backups hero, and the notifications panel share the same refined card, badge, and pill language for one consistent look.

### Added — notifications

- New `notifications-store` SQLite crate with FIFO eviction at 200 entries and a six-command Tauri surface: `list_notifications`, `mark_notification_read`, `mark_all_notifications_read`, `dismiss_notification`, `push_notification`, `notifications_unread_count`.
- Bell in the top bar with an unread badge and a dropdown inbox. Seven notification kinds: apply succeeded, apply failed, apply cancelled, app update available, catalog update available, library scan failed, catalog refresh failed.
- Emitters wired at the source. The update check, the catalog-refresh diff, and the scan pipeline now push persistent entries. Clicking an app-update notice reopens the update banner; a catalog notice jumps to the Catalog view.

### Added — UI/UX

- Command palette (`Cmd+K` / `Ctrl+K`) with fuzzy match over the full command set, persisted recents, and category filters (`All` / `Navigate` / `Action` / `Settings`).
- Keyboard-shortcuts overlay (`?`) grouped by scope: Global, Library, Drawer, Modal, Palette.
- Library view-mode toggle (Grid / List), density toggle (Compact / Comfy), and a sort selector, all persisted in `ui_prefs`.
- Real, original launcher logos for Steam, Epic Games, GOG Galaxy, Ubisoft Connect, Xbox, EA, and Battle.net on the Settings detection card.
- Catalog footer pinned to the bottom of the view with a live status dot and a `Refresh now` action.
- Apply progress modal rebuilt around failures: filter chips (All / Failed / Running / Done), a per-stage timeline (Download → Hash → Signature → Backup → Replace → Verify), per-group collapse, and an action rail — `Retry all failed`, `Allow unsigned & retry`, `Copy report`, `Cancel all running`. Same-error rows inside a group dedupe with an `× N files affected` badge.

### Added — backend

- Tray badge with the in-flight apply count and a right-click → Show progress entry that opens the modal even from the tray.
- Batch apply: one click runs every selected target through a single back-end command that shares the download cache, bounded by a configurable `apply_concurrency` (1–4, default 2).
- New Network section in Settings → Advanced: `retry_attempts` (3), `chunk_timeout_secs` (60), `connect_timeout_secs` (10), `download_cache_ttl_secs` (300).
- Centralized rolling-file logging. Every run writes a daily log to `%USERPROFILE%\DLSSync\Logs\dlssync.log.<date>` alongside stdout; the Logs folder is no longer empty. The level filter honors `RUST_LOG`.
- `Report a problem` (About) and `Report issue` (apply modal) open a pre-filled GitHub issue with the app version, OS, and the last 40 log lines attached. Nothing is sent until you submit; the body is capped to stay within the URL limit.

### Changed

- `AppSettings.ui_prefs` extended (backward-compatible via serde defaults) with view-mode, density, sort, backups group-by, active settings tab, and recent palette commands. Legacy settings files load unchanged.
- Backups snapshots now carry an explicit state: Active backup, Restored (still re-restorable while the snapshot is on disk), and Snapshot missing (the file was removed outside the app). The hero and group rows surface a count for each.

### Fixed

- **Backups → Reveal snapshot file** now opens the snapshot's folder with the file selected instead of always landing in Documents. The reveal command quoted the entire `explorer /select` argument, so any path containing spaces — every Steam install — was unparseable and Explorer fell back to its default folder. The path is now quoted on its own, and a snapshot that is no longer on disk falls back to opening its parent folder.
- The game detail drawer no longer leaves a dead gap on the right of every other view. Opening it set a global flag that reserved the drawer's width app-wide; switching tabs unmounted the drawer but kept the reserved space. The drawer now closes when you leave the Library, and below 1300 px it overlays the content with a scrim instead of squeezing it. The Library hero band reflows through a container query so its actions stack below the summary when the column narrows rather than overlapping it.
- The apply progress modal no longer leaves a detached pane behind when `View backups` navigates away. It is mounted at the app root instead of inside the Library view, so a view change can no longer orphan its transition.
- NVIDIA single-DLL-in-ZIP families (`nvngx_dlss.dll`, `nvngx_dlssg.dll`, `sl.dlss_g.dll`) no longer fail with `size mismatch` when the upstream archive compresses the DLL. The compressed-length check is deleted outright; integrity is now exclusively the SHA-256 over the extracted DLL.
- Intel XeSS applies no longer fail halfway through. The four sibling DLLs that share one `XeSS_SDK_<v>.zip` now download the archive exactly once instead of four times — roughly 4× faster and about 50 MB of traffic instead of 200 MB.
- Large downloads no longer time out mid-stream. The timeout applies per chunk, not per request, so a transfer keeps going as long as bytes keep arriving, including across a PC sleep.
- Transient GitHub CDN failures (TCP reset, truncated body, 503, 429) retry up to three times with backoff. A 404 or other permanent failure still fails fast.
- Cancelling an apply or hard-killing the app no longer leaves staging folders behind in the backups directory; leftovers older than 24 hours are reclaimed on the next launch.

### Security

- Zip extraction rejects entries whose path contains `:` (NTFS Alternate Data Stream), control characters, surrogate pairs, or `..` components, with a 200 MiB per-entry and 1 GiB per-archive cap against zip-bomb manifests.
- No new outbound endpoints. The auto-update payload is still verified against the embedded Ed25519 public key before extraction.

[1.2.0]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.2.0

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
