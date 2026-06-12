# Changelog

All notable changes to DLSSync are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.6.9] - 2026-06-12

The integrity-hardening release. This is the public v1.6.9 line for the v1.6.8 mouse side-button navigation polish plus the security/integrity follow-up: it closes the active esbuild Dependabot advisory without forcing a risky frontend framework migration, restores the generated manifest/schema contract that was breaking the hourly DLSSync-Manifest pipeline, and tightens the release docs around the signed catalog path.

### Added

- Shipped the v1.6.8 mouse side-button navigation polish in the public v1.6.9 build: button 3 moves back through DLSSync app state and button 4 restores the next state when one exists.

### Security

- Forced the transitive `esbuild` toolchain dependency to `0.28.1`, the patched line for GHSA-gv7w-rqvm-qjhr, through the root `package.json` pnpm override instead of leaving Vite/Vitest on a vulnerable binary resolver.
- Kept the signed manifest contract strict: empty dynamic anti-cheat binary rows are no longer serialized into the generated public catalog, preventing schema drift from breaking the manifest publisher.

### Changed

- Release metadata, README, docs, and test index now point at v1.6.9 as the current security/integrity line.
- The production frontend build now targets the Tauri WebView2 baseline directly instead of falling back to Safari when `TAURI_PLATFORM` is absent.
- The anti-cheat docs now document how optional manifest binary signatures layer over the static detector without emitting an empty field.

### Fixed

- The hourly DLSSync-Manifest workflow no longer receives a generated `anti_cheat_binaries: []` field that the public schema rejects.
- Patched esbuild no longer tries to downlevel the Svelte runtime to an irrelevant Safari target during standalone frontend builds.

## [1.6.8] - 2026-06-12

The navigation-polish release. DLSSync now respects the back and forward side buttons on gaming mice, but keeps that movement inside the app instead of handing it to Chromium history. You can jump back to the previous menu, restore the next app state when one exists, and reopen or close the game-detail drawer through the same history stack.

### Added

- Mouse side-button navigation across DLSSync app state: button 3 goes back and button 4 goes forward through views and game-detail drawer open/close states.
- A small app-navigation history contract with unit coverage for back/forward movement, duplicate-state suppression, forward-branch dropping after new navigation, and mouse-button mapping.

### Changed

- The app shell now intercepts recognized side-button `mouseup` events and writes the target state back into `currentView` and `drawerGameId`, so browser history is never used for internal navigation.
- README and release-marketing docs now point at v1.6.8 as the current public release line.

### Fixed

- Mouse users no longer have to return to the sidebar or keyboard shortcuts just to move back to the previous DLSSync menu or detail-drawer state.

## [1.6.7] - 2026-06-10

The vendor-parity, trust, and color release. AMD one-click driver install works now — it was quietly broken before. The catalog is cryptographically enforced: a tampered manifest is rejected, and an offline first run still loads a signed catalog instead of a blank screen. Two full security passes closed holes around the driver installer, backup restore, elevated restores, and downloads. FSR and XeSS now update as coherent multi-DLL sets in one atomic click — with a hardware gate so FSR 4 is never pushed onto a GPU that can't run it. And the whole interface learned to speak in color: vendor-brand tech badges, a status hero in the Library, traffic-light state everywhere, and game art that tints the app around it.

### Security

- Manifest signatures are now enforced. A catalog whose Ed25519 signature does not verify is refused, and the app falls back to the last trusted copy on disk or a signed manifest bundled in the app — so a CDN hiccup or a tampered file can never feed you the wrong DLLs, and an offline first run still works.
- Closed an argument-injection path: the value passed to the elevated (Administrator) driver installer is now strictly validated and quoted, so a crafted update id cannot smuggle extra flags into the elevated process.
- Backup restore is fenced in. It refuses to write outside the backup store, through a symlink, or onto a system file, and it verifies the backup's hash before restoring.
- Driver downloads only happen from official NVIDIA, AMD, and Intel domains. An arbitrary download URL is rejected.
- Hardened the rest of the surface: notification links are sanitized to known hosts, scan targets can't be pointed at your whole drive, the catalog download can't be tricked into a giant allocation, and every CI action is pinned to an exact commit.
- The elevated system-driver restore validates and safely quotes its backup path before the Administrator helper sees it, and it refuses paths outside the backup store or through symlinks.
- The driver installer is downloaded into a randomized staging folder and verified-then-launched from that same path, closing the window where a local attacker could swap the file between the check and the run.
- Every download now has a hard total-size cap and an overall deadline, so a misbehaving server can't slowly stream gigabytes into your disk or stall an install forever.
- DLL paths arriving from the interface are re-checked at the apply boundary — extension, location, and system directories — before any file is touched, and a driver whose signature chain skipped revocation checking is labeled instead of silently trusted.

### Added

- AMD one-click driver install. DLSSync now resolves the real Adrenalin installer for your card's branch and installs it — previously the AMD path had no installer link and silently failed.
- NVIDIA Standard-vs-DCH driver detection, so the small set of users on Standard drivers stop being handed a DCH installer that won't take.
- Intel drivers now match your Windows version, so a Windows 10 machine is never offered a Windows 11-only package.
- Silent driver installs for NVIDIA and Intel — a routine update no longer pops the full vendor installer wizard.
- A bundled NVIDIA GPU database and the signed fallback catalog, so driver lookups and the catalog both keep working if a source is briefly unreachable.
- Anti-cheat detection can pick up new or renamed engines from a catalog refresh, instead of waiting for an app update.
- Unrecognized GPUs now get vendor-neutral recommendations (XeSS, DirectStorage, Reflex, Streamline) instead of nothing.
- One-click FSR and XeSS set updates. AMD's FSR and Intel's XeSS ship as matched multi-DLL sets (loader, upscaler, frame generation), and swapping only one member breaks games — DLSSync now updates the whole set as a single atomic transaction from the game panel: all files succeed together or everything rolls back.
- An FSR 4 hardware gate. FSR 4 upscaling only runs on AMD RDNA4 cards (Radeon RX 9000 series); on anything else the set action is locked with an explanation instead of letting you install DLLs the GPU can't use — checked in the interface and enforced again in the backend.
- Reverted-swap detection. When a game update quietly rolls your swapped DLL back to the old version, DLSSync notices on the next scan and raises a notification — the game shows as outdated again and one click (or background auto-apply) re-applies your version.
- A "Managed by NVIDIA App" notice in the game panel when NVIDIA's own per-game DLSS override is active and DLSSync has no preferences for that game, so the two tools never silently fight over the same files.
- A Library status hero: the pending-update count as a big color-coded numeral, a bar segmented by vendor so you see at a glance whose tech needs updating, and Games / Up to date / Protected counters (Protected = games with restore points).
- A driver health summary strip at the top of Drivers — one chip per GPU with a status dot, plus a system-components chip — and a DLSS preset recommendation matched to your GPU generation (Preset K for RTX 20/30; K plus L/M Performance presets for RTX 40/50).
- Catalog freshness at a glance: the manifest timestamp is green when fresh, amber when the feed has gone stale.

### Changed

- Library cleanup: one primary Apply action instead of three competing buttons, per-game tech badges that highlight what's outdated versus up to date, a shortcut to reveal hidden games, and a designed cover for games with no art.
- The game detail panel was split into focused pieces (hero, feature list, action bar). The footer buttons are labeled, the selection checkboxes show a keyboard focus ring, and the scrollbar appears on hover so long feature lists are obviously scrollable.
- Per-game DLSS presets are keyed to the full executable path, so two games that share an executable name no longer share each other's overrides.
- Every dialog now traps keyboard focus and restores it on close, with the correct screen-reader roles.
- The background scan holds its cadence instead of drifting later each day on a slow machine.
- Settings, Drivers, Backups, About, and Catalog each got a pass for clearer grouping, copy, and progressive disclosure.
- The interface speaks in color now. Tech badges carry their vendor's brand color (NVIDIA green, AMD red, Intel blue, Microsoft purple) with contrast-checked inks in both themes; state dots mark outdated (amber), current (green), and anti-cheat (red); counters and numerals are toned by meaning instead of all reading gray.
- The focused game's cover art now tints the app backdrop with its dominant color — a subtle radial accent that follows what you're looking at, calmer in light mode, and still when you've asked Windows for reduced motion.
- Accessibility pass across all six views: every checkbox and toggle has a real accessible name, game cards and list rows are keyboard-reachable without nested-button traps, each game-panel row has a visible "More actions" button as the keyboard twin of right-click, and axe reports zero critical or serious issues in both themes.
- The end-to-end test suite now runs against an isolated data directory, so a test run can never touch your real library, settings, or backups.

### Fixed

- Pinned DLLs no longer show up as "outdated" in the library, sidebar, and Apply-all count.
- A background auto-apply that lands while you're applying updates by hand no longer wipes the progress modal.
- Efficiency mode stays off until you turn it on — it used to enable itself on a first run.
- The catalog cache survives a power loss mid-write instead of blanking the library on the next launch.
- The notification vendor logos and CDN routing now agree on one source of truth per technology, and a Spanish notification typo was corrected.
- The DLSSync logo chip rendered as a dark blob in light mode; it now follows the theme.
- The Drivers view showed raw translation keys for the system-components summary, and About's "What's New" leaked the date into the first line — both fixed, in English and Spanish.
- About's header buttons overflowed the window at narrow widths instead of wrapping.
- Cancelling an apply that fails now tells you instead of doing nothing; finished apply progress is pruned after a few minutes so a long tray session can't accumulate state forever; background scan ticks no longer race a manual apply or pop phantom "found games" toasts; and notification de-duplication is language-independent, so switching languages doesn't re-spam old notifications.
- The app shell now owns scrolling consistently: long views keep an elegant token-styled scrollbar, the root window no longer exposes the default right-edge scrollbar, and the detail rail no longer fights the main content for scroll space.

## [1.6.6] - 2026-06-07

Quality-of-life on the X button, the apply modal, and Backups. The X minimizes to the system tray for new installs while your saved choice still wins. The Apply Progress wall is gone: close the modal mid-apply, navigate to Catalog or About, then pick it back up from the activity dock. And the date-grouped Backups view divides itself into month headers, so months of history read at a glance instead of a flat scroll.

### Added

- A token-driven `<CounterPill>` component shared by every sidebar counter so the badges line up the same way everywhere.
- Three intent-named sidebar groups (Library, Catalog & Drivers, History) plus the existing General. Same icons, same view names, clearer headers.
- A bundle-size contract test that fails the build if the main JS chunk exceeds 250 KB gzip or the CSS exceeds 75 KB gzip.
- A rotating chevron on the language switcher tied to `aria-expanded` so the menu state reads correctly to screen readers and to the eye.
- A month header divides the Backups date-grouped view at every month boundary, so multi-month histories read at a glance instead of a long flat list.
- An axe-core accessibility contract over the core components (CounterPill, BrandMark, Checkbox, Toast) that fails the build on any critical or serious WCAG violation.

### Changed

- Close-to-tray is on by default for new installs. An explicit `false` from a prior config is preserved — never silently re-enabled.
- The Apply Progress modal can be closed while applies are still running. Click the X, hit Escape, or click outside, and it minimizes to the activity dock. The apply keeps going, the rest of the app is yours. A one-time hint shows you where the dock is the first time.
- The apply pipeline's pure decisions (error classification, signature-error hinting, version-major parsing, the Streamline cross-major ban check, the failure-outcome shape) moved into their own module with a focused 14-test cargo suite. The 400-line apply hot path stays the orchestrator; the decisions get their own contract.
- Driver history and Catalog versions flyouts share a `<FlyoutShell>` primitive now. Backdrop, dialog frame, vendor-accent routing, and the Escape-close behavior live in one place instead of repeated in each consumer.

### Fixed

- Two concurrent driver checks used to collect WMI/DXGI system info twice in parallel under a stale read-lock pattern. A coordinator now serializes the collection so it runs exactly once, and the cached value is reused.
- Large DLL copies during apply no longer block the tokio runtime — every `std::fs::copy` is wrapped in `spawn_blocking` so download-progress emits, cancellation signals, and tray pings stay responsive even while the file copy runs.

## [1.6.5] - 2026-06-03

DLSSync can keep your DLLs current on its own now — a background daemon that scans on a schedule, sits in the tray, and warns you before you touch a game that can ban you for it.

### Added

- Background updates. Turn it on and DLSSync rescans your library on a schedule (every 1 to 168 hours) even with the window closed, shows how many games have updates ready in the tray, and raises a Windows notification when something is waiting. It can close to the tray instead of quitting, start with Windows minimized, and apply everything in one click from the tray, the notification, or the Library header. Optional auto-apply always skips anti-cheat games and makes a backup first. Off by default.
- A ban-risk warning at the moment you apply. Games protected by Easy Anti-Cheat, BattlEye, or Riot Vanguard now show a risk chip next to Apply and ask for one explicit confirmation before any DLL is replaced — the warning lands at the apply step, not buried in a panel.
- Manifest signature verification (Ed25519) against a key baked into the app. Verification ships now; fail-closed enforcement is staged for a later release, once the signed manifest is live across the CDN.

### Changed

- Notification logos come from the component being updated, not a guess at the wording — so a DLSSync release no longer shows up wearing NVIDIA's logo, and an FSR or XeSS update wears the right one.
- The close-to-tray and start-with-Windows switches moved into the new Background updates section; the Performance section keeps the efficiency (EcoQoS) toggle.
- In release builds the catalog URL can no longer be redirected by an environment variable.

### Fixed

- Duplicate driver notifications are gone — dedup is enforced in the database instead of racing in memory — and the ones that had piled up are pruned the next time you launch. Apply notifications also stop being swallowed after the first one per game.
- Authenticode checks validate the whole certificate chain for revocation, falling back only when the machine is offline.
- ZIP extraction is bounded by the bytes actually written rather than a size the archive claims, a single failed download no longer poisons that file's cache for the rest of the session, a cancelled apply stops sooner, and a failed item in a batch now reports which one.
- Updated the test runner to clear a security advisory.

## [1.6.4] - 2026-06-02

A visual overhaul. The game detail panel, the library filters, and the About, Settings, and Backups pages were rebuilt, the whole app reads correctly in light mode, and notifications stop repeating themselves and start telling you something worth reading.

### Added

- A "DLL updates ready" notification that sums up what is waiting — "8 updates ready in 3 games" — and opens the Library when you click it, so you are not counting per game.

### Changed

- The game detail panel is part of the window now, not a card floating beside it. The title bar runs the full width with minimize, maximize, and close in the corner where they belong, the whole top edge drags the window, and the panel keeps a fixed header and footer so the cover, the anti-cheat warning, and the Apply button stay put while the rest scrolls. The thin line above the cover takes its color from the game's art.
- Library filters went from two rows of scattered pills to two compact menus, Launcher and Status, that never wrap; sort, view, and density sit together on the right.
- About, Settings, and Backups were redesigned for clearer hierarchy and tidier stat cards, and every tab shows one title instead of two.
- Notifications carry the vendor's logo — NVIDIA, AMD, Intel, Microsoft — in place of a generic glyph.

### Fixed

- Light mode is legible everywhere. White text over a bright cover no longer washes out, and the version picker, command palette, and notifications no longer bleed the blurred cover art into a reddish smear behind the content.
- The same "new driver available" alert was re-posted on every launch; identical notifications collapse to one now.
- The command palette search box dropped its boxy focus outline, and the page title no longer appears twice at the top of every tab.

## [1.6.3] - 2026-06-01

DLSSync speaks your language now, and it stops standing between you and the DLSS Enabler.

### Added

- Language support. The whole interface switches between English and Spanish from a globe button at the bottom of the sidebar, and your choice survives a restart. Every word on screen lives in a plain JSON file, so adding a language is a copy-and-rewrite job with no code and no build tools — there is a step-by-step guide for translators in [docs/translations.md](docs/translations.md).

### Fixed

- Under a detected DLSS Enabler, DLSSync updates the Streamline plug-in set within the same major version again, instead of locking it as "Managed by Enabler". The Enabler needs Streamline 2.11 or newer but never updates Streamline itself, so the previous lock kept you from giving the Enabler the version it asks for. Same-major updates are offered again; a cross-major swap — your 2.x set jumping to the driver's 310.x line — is still blocked, because mixing majors crashes the game on launch. Raised on Nexus for Subnautica 2.

## [1.6.2] - 2026-05-30

Two small additions to help DLSSync find its people: a support card you can turn off, and a Nexus Mods link in About.

### Added

- A support card. After an update succeeds, DLSSync may show a card in the bottom-left — star it on GitHub, endorse it on Nexus Mods, or share it with a friend. It blocks nothing and never disappears on a timer. Close it, or pick "Don't show again" to retire it for good. It comes back from Settings → General → Show the support card whenever you want it.
- A "Help DLSSync grow" section in About, with a live GitHub star count, a Nexus Mods endorsement link, and a share button — plus a Nexus Mods link in the About header.

## [1.6.1] - 2026-05-30

DLSSync updates NVIDIA's Streamline plug-ins, and reads them correctly across both of their version schemes. In modern games DLSS Frame Generation is two files — the NGX runtime `nvngx_dlssg.dll` on the 310.x line and the Streamline plug-in that drives it, `sl.dlss_g.dll`, on the 2.x line. The Streamline plug-ins update as a matched set from NVIDIA's own signed SDK, applied all-or-nothing, and only when your installed plug-in is on the same version line as the SDK — a driver-managed build is left alone, so your NVIDIA App overrides keep working.

### Added

- Update the Streamline plug-in set in one step. When a game ships NVIDIA's Streamline plug-ins — `sl.dlss.dll`, `sl.dlss_g.dll`, `sl.dlss_d.dll`, and the `sl.interposer`/`sl.common`/`sl.pcl` runtime — the game-detail page offers a single "Update Streamline set" action that swaps the whole matched set to one official SDK version. The plug-ins are version-locked: a half-updated set crashes the game on launch, so the update is atomic — every file installs, or the previous set is restored. The binaries come from NVIDIA's own signed SDK release at apply time, verified by Authenticode; DLSSync hosts nothing. Gated behind "NVIDIA Streamline plug-ins (advanced)" in Settings → Update preferences, and skipped for games a DLSS Enabler manages. NVIDIA's driver already updates these in many games, so this is for titles that ship with that turned off, a pinned version, or an offline machine.
- The "Update Streamline set" action carries a note: updating Streamline can change how NVIDIA App global overrides apply, per-game overrides keep working, and you can restore the previous set from Backups.

### Fixed

- DLSSync no longer offers a bogus 310.x update for a 2.x Streamline plug-in. It reads which build each `sl.*` plug-in is — the GitHub SDK stamps the 2.x version, NVIDIA's driver stamps the 310.x version — and offers an update only when the installed plug-in is on the same line as the catalog (2.x) and older. A driver-managed 310.x plug-in is left untouched, which keeps NVIDIA App global preset and ratio overrides working. Reported on Nexus for Subnautica 2.
- DLSS Enabler detection catches its strongest marker, `nvngx-wrapper.dll`, plus the enabler log and ASI loader, so a game it manages is recognized even when the older marker files are missing.
- Under a detected DLSS Enabler, the genuine Streamline plug-ins (`sl.interposer.dll`, `sl.common.dll`, `sl.pcl.dll`, `sl.reflex.dll`) show a "Managed by Enabler" label and cannot be selected, matching the on-screen notice that DLSSync leaves them alone. They used to show an "Update" badge that apply would refuse anyway. The `nvngx_*` runtime DLLs still update.

## [1.5.2] - 2026-05-29

A reliability and catalog release. The in-app catalog is complete again — DirectStorage and the full AMD and Intel upscaler history are back — and DLSS updates no longer swap the version-locked Streamline runtime that could crash games such as Starfield. The System & Components driver updater installs without hanging, snapshots each driver before updating so you can roll it back, and shows the older and latest versions of every component driver. It also carries the notification, command-palette, and game-detail work from the interface pass.

### Added

- System & Components shows each component's installed driver version next to the older versions still cached on your PC and the latest available one, read from the local driver store.
- Before a system or component driver update, DLSSync exports the current driver and sets a System Restore point, then records it in a new, filterable System Drivers section in Backups where you can roll any driver back.
- A note on the Drivers tab explains why a driver install asks for Administrator: Windows installs per-machine drivers only with elevation, so DLSSync stays unelevated and runs a signed helper with your approval.
- Release notifications now carry the version, a short summary of the release notes, and one-click links to both the GitHub release and the Nexus Mods page — instead of only the changelog heading.
- The notification center covers more events: GPU driver updates, system and component driver updates, and restored backups each post their own entry. Any notification with a link opens it externally.
- Command palette: a per-command icon, a keyboard-shortcut chip (for example, G then L for Library), grouped sections (Recent, Navigate, Action, Settings), and highlighting on the characters your query matched.

### Changed

- DirectStorage (dstorage.dll with dstoragecore.dll) and the split FSR runtime are handled as matched sets, and the catalog build no longer overwrites the older AMD, Intel, and DirectStorage versions it used to drop.
- After a DLL swap, DLSSync re-hashes the file it wrote and rolls back automatically when it does not match, and reports a locked file (game still running) separately from a real failure.
- The game-detail view is now a full page instead of a slide-over panel. Opening a game shows a spacious page with a hero banner, status, update counts, the feature list, and a docked action bar; the window controls, search, notifications, and theme toggle stay visible and usable. "Back to Library" or Esc returns.
- The command palette opens as a clean top-right panel — no full-screen dim — with an integrated search field.
- System & Components (Drivers tab) copy rewritten to be plain and concrete.

### Fixed

- DLSS no longer swaps NVIDIA's version-locked Streamline runtime (the sl.* DLLs) across SDK major versions, or when an injector mod such as DLSS Enabler or OptiScaler is present — the cause of immediate crashes in games like Starfield and Subnautica 2. The upscaling DLLs (nvngx_dlss and the rest) still update normally.
- "Not in catalog" no longer shows for DirectStorage and other technologies that are in fact covered; the catalog now carries them.
- A System & Components driver install no longer hangs after the Administrator prompt and then disappears. It shows real progress, times out instead of freezing, and always ends with a clear result — including the per-machine "access denied" (0x80240044) case.
- Resetting a DLSS override no longer creates an empty per-game profile that inherited the global preset. A game without an override now follows its own configuration.
- The notification center is frosted again — the page behind it no longer shows through. The panel was moved out of the top bar so its blur applies to the page instead of being clipped.
- The detail page's action bar no longer clips over content mid-scroll; it is an opaque docked bar.
- A notification's link survives an app restart. It is stored with the notification, and notifications saved before this release migrate automatically.

## [1.5.1] - 2026-05-28

A driver-detection accuracy release. GPU driver resolution is now keyed on the PCI device id for every
vendor, so laptops with integrated Intel graphics alongside a discrete card get the correct driver instead
of the Arc desktop package, and install progress no longer breaks when you switch tabs.

### Fixed

- Intel driver detection resolves the exact driver for the installed GPU by matching its PCI device id against the catalog's per-package hardware-id list, instead of always serving the Arc desktop driver. Integrated Intel graphics — including 6th–10th-gen parts on the 31.x branch and 11th–14th-gen parts on their own branch — no longer trigger the installer's "exit code 8" (no compatible device) and no longer open the generic Arc page when you click Release notes.
- Driver download and install progress survives switching tabs. The state moved to a shared, app-level store, so leaving the Drivers tab mid-download no longer cancels the progress or corrupts the animation.
- On systems with more than one GPU — an integrated plus a discrete card, or two cards from the same vendor — each adapter's installed driver version is matched by PCI hardware id rather than by name, so versions are no longer assigned to the wrong card. Duplicate adapter enumerations are collapsed.

### Added

- Device-id-driven resolution for all three families: Intel matches the catalog hardware-id list, AMD maps the device id to its driver branch (RDNA3+, RDNA1/2, Polaris/Vega) before falling back to the model name, and NVIDIA notebook ("Laptop GPU") and desktop products resolve to distinct drivers.
- A clearer Drivers tab: an "Open download page" action for AMD, a "Find my driver" link when a GPU's driver cannot be resolved, an `aria-live` install-progress region, and visible keyboard focus rings.

### Changed

- AMD opens its official download page instead of a constructed installer URL. The direct `.exe` is gated behind a license prompt and its filename changes per release, so a fabricated link was unreliable; version and changelog detection are unchanged.
- Vendor installer exit codes are reported with a readable message. Intel's "no compatible device" (exit code 8) now explains the GPU may be OEM-locked or need a different driver branch, pointing to the manufacturer or Windows Update.

[1.6.9]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.9
[1.6.8]: https://github.com/xt0n1-t3ch/DLSSync/compare/v1.6.7...981c0c962ca1b53b34f2ba87a0726b459162c4ff
[1.6.7]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.7
[1.6.6]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.6
[1.6.5]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.5
[1.6.4]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.4
[1.6.3]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.3
[1.6.2]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.2
[1.6.1]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.6.1
[1.5.2]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.5.2
[1.5.1]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.5.1

## [1.5.0] - 2026-05-27

The public release of the GPU driver updater, the NVIDIA DLSS overrides and the anti-cheat work
detailed in the 1.3 and 1.4 sections below, plus per-GPU driver history, a detection overhaul, and
a redesigned Library, Drivers tab and game drawer. Skips 1.3 and 1.4 as public builds.

### Added

- Per-GPU driver history: an "All versions" flyout lists every driver known compatible with the card — NVIDIA returns 50, AMD and Intel their full branch. A WHQL-only filter hides Beta and Studio builds and disables itself when the vendor publishes none for that GPU.
- Per-game protection detection merges a local scan with a dataset built from PCGamingWiki and bundled offline. The scan reads anti-cheat binaries by name (Easy Anti-Cheat, BattlEye, Riot Vanguard, nProtect GameGuard, XIGNCODE3, EA AntiCheat, PunkBuster, HoYoProtect) and inspects the main executable for protector fingerprints — Denuvo's section cluster, VMProtect, Themida, Steam CEG. Games resolve by Steam app id or by a normalized name.
- Shared custom checkbox and dropdown controls replace the native ones in the DLSS override panel and the version browser.
- F12 opens DevTools in any build.

### Changed

- The game detail drawer is redesigned: a layered-gradient scrim in place of the blur, a hero accent stripe, a hairline top-edge highlight, and a calmer section rhythm.
- The Library update summary is rebuilt around a one-line headline, colored per-technology tags, and Review / Apply all actions.
- The Drivers card actions are rebuilt around one "Update to vX" button carrying the download size, plus icon-only Release notes and All versions controls.
- FSR and XeSS carry their full version history from the DLSS-Swapper manifest — FSR Upscaling from 1 to 9 tracked versions, XeSS from 11 to 15 — instead of the sparse FidelityFX SDK feed.
- The version browser lists every DLL family that maps to a feature (FSR Upscaling shows both the DX12 and Vulkan libraries), de-duped and newest-first.
- The Catalog is sectioned with a GPU-driver overview on top and an Upscaling Libraries & Technologies grid below, replacing the column flow that ran the vendor cards together.
- The anti-cheat "Learn more" link opens the game's PCGamingWiki page — by Steam app id when known, by name search otherwise.

### Fixed

- The WHQL-only filter in the driver history flyout now works. NVIDIA history is fetched unfiltered, so Beta and Studio drivers surface alongside Game Ready instead of being the only channel ever returned.
- The Catalog footer no longer overlaps the cards at the bottom of the page.
- The Settings "DLSS Overrides" pointer button is no longer cramped against its description.

### Security

- The driver installer is launched only after `WinVerifyTrust` validates the file digest and the certificate chain, then the signer subject is matched against the vendor allowlist. The previous check read the embedded certificate's subject name without verifying the signature, so a binary carrying a self-signed certificate with a vendor subject would have passed; the new gate rejects it.
- The downloaded installer filename is taken from the URL as a single sanitized path component (path separators, drive/ADS colons and `..` are rejected, query and fragment stripped), so a crafted download URL cannot write outside the driver cache directory.

[1.5.0]: https://github.com/xt0n1-t3ch/DLSSync/releases/tag/v1.5.0

## [1.4.0] - 2026-05-26

An end-to-end rework of the Drivers tab, the DLSS override controls, and a new per-game
anti-cheat warning.

### Added

- Per-game anti-cheat detection: a local scan of the install folder for known anti-cheat binaries (Easy Anti-Cheat, BattlEye, Riot Vanguard, nProtect GameGuard, XIGNCODE3, EA anticheat, PunkBuster, HoYoProtect) merged with a community dataset distilled from AreWeAntiCheatYet and bundled into the manifest, matched by Steam app id or name. New crate `anticheat-detect`.
- A ban-risk warning in the game detail drawer above both apply paths — the DLL swap and the DLSS override — naming the detected anti-cheats. It warns; it never blocks.
- The Drivers tab now shows a "What's new" changelog per driver (highlights and fixed-issue notes) and a "Release notes" link to the official per-vendor page.

### Changed

- The Drivers tab is now the hub for GPU drivers and DLSS. Global DLSS preset and frame-generation overrides moved here from Settings → Advanced; per-game overrides stay in each game's detail drawer.
- Intel and AMD driver resolution is live: Intel reads the DSA software-configuration catalog, AMD reads the GPUOpen version table and maps the public Adrenalin release to the installed driver-store version per GPU generation. AMD degrades to "Open driver page" where a per-GPU installer URL is not certain.
- The DLSS override panel is redesigned. Every preset and frame-generation option carries a plain-language description and a link to its canonical NVIDIA source, and the Dynamic Multi Frame Generation gate is corrected to Game Ready 595.97.

### Fixed

- The "Release notes" / driver page button now opens in the browser. It previously called the filesystem opener with an `https` URL and silently did nothing.
- The DLSS override panel no longer renders with overlapping labels; the layout was colliding with shared global class names.
- NVIDIA driver release dates now parse the live `Tue May 26, 2026` format instead of being dropped.

## [1.3.0] - 2026-05-26

GPU driver updating for NVIDIA, Intel and AMD, plus DLSS preset and frame-generation
overrides applied through the NVIDIA driver profile.

### Added — GPU driver updater

- New `Drivers` tab lists every detected GPU with its installed driver and the latest available version, resolved live per vendor. NVIDIA resolves through the public GeForce driver lookup API keyed by the GPU product id from the `ZenitH-AT/nvidia-data` map; Intel and AMD sit behind the same `DriverSource` trait for later activation.
- `Download & install` fetches the official vendor installer, verifies its Authenticode publisher signature against the NVIDIA/Intel/AMD allowlist, then launches the vendor's own installer, which self-elevates through UAC. DLSSync never elevates itself.
- Real-time install tracking reports Downloading (byte-accurate) → Verifying → Launching → Installing → Completed/Failed/Cancelled in the Drivers tab, classified from the installer exit code.
- New crates: `driver-catalog` (extensible `DriverSource` registry where GPU is the first device class) and `driver-install` (streaming download, signature gate, install state machine).

### Added — DLSS preset and frame-generation overrides (NVIDIA)

- Global overrides in Settings → Advanced and per-game overrides in each game's detail drawer, matching the NVIDIA app's split.
- Force the DLSS Super Resolution model preset (A–M or Recommended), the frame-generation mode (Fixed or Dynamic), the multi-frame multiplier (2×/3×/4×), and a Dynamic target frame rate.
- Applied by writing the NVIDIA driver application profile through NVAPI DRS — the same mechanism the NVIDIA app uses, not DLL injection. Each override reverts in one click.
- New crate `nvapi-drs` calls `nvapi64.dll` through its `nvapi_QueryInterface` dispatch and needs no administrator rights.

### Changed

- The security model now separates the DLL-swap path (no driver, no kernel hook, no injection) from the new NVIDIA-profile override path, which writes a reversible driver application profile.
- The outbound network surface adds the GeForce driver lookup API, the `ZenitH-AT/nvidia-data` map, the Intel driver catalog and the AMD driver host. Every request stays unauthenticated with zero telemetry.

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
