# DLSSync test suite

Centralized tests for the whole app. Frontend logic/integration runs on Vitest (happy-dom);
backend runs on `cargo test`. Run both before shipping.

## Run

| Layer | Command | What it covers |
|:---|:---|:---|
| All (pre-ship) | `task test` | Frontend vitest + full Rust workspace |
| Frontend | `pnpm --filter dlssync-frontend test` | Vitest unit + integration + component + contract (happy-dom) |
| Frontend (watch) | `pnpm --filter dlssync-frontend test:watch` | Same, watch mode |
| Backend | `cargo test --workspace` | Every Rust crate + Tauri command surface |
| Types | `pnpm --filter dlssync-frontend check` | svelte-check, 0 errors / 0 warnings |
| End-to-end | `pnpm test:e2e` | Playwright over CDP drives the real WebView2 app (`tests/e2e`) |

Vitest config: [frontend/vitest.config.ts](../frontend/vitest.config.ts). Tauri APIs are mocked in
[tests/setup.ts](setup.ts) so store/logic modules import cleanly outside a WebView.

## End-to-end (`tests/e2e/`)

`@playwright/test` drives the real app's WebView2 over the Chrome DevTools Protocol. The CDP port
(`9333`) is opened only in debug builds — [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) sets
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` under `#[cfg(debug_assertions)]`, so release binaries never
expose it. [global-setup.ts](e2e/global-setup.ts) builds a hermetic debug binary
(`tauri build --debug --no-bundle`, bundled `frontend/dist`, no Vite dev server);
[fixtures.ts](e2e/fixtures.ts) clears stale instances, launches the app, waits for CDP, connects via
`connectOverCDP`, and attaches a console/exception collector so every spec inherits a zero-error
guard. Data-dependent checks (`test.skip`) gate gracefully when the live machine lacks the data.
App version is read from `package.json` at runtime — never hardcoded. Config:
[playwright.config.ts](e2e/playwright.config.ts) (HTML report + traces on first retry).

## Frontend — unit (`tests/unit/`)

| File | Module under test | Coverage |
|:---|:---|:---|
| [formatHuman.test.ts](unit/formatHuman.test.ts) | `lib/formatHuman` | byte/speed/eta/duration/elapsed/percent formatting; null·NaN·Infinity·negative·boundary tiers |
| [relation.test.ts](unit/relation.test.ts) | `lib/relation` | version compare, sha short-circuit, vendor-sha match, target resolution, `gameStatusFromRecords` across outdated/up-to-date/no-dlls/scan-failed/disabled |
| [applyErrorClass.test.ts](unit/applyErrorClass.test.ts) | `lib/applyErrorClass` | all 9 error classes, precedence (cancelled→network→signature…), case-insensitivity, action routing, label/tone tables |
| [labels.test.ts](unit/labels.test.ts) | `lib/labels` | family→vendor/group/catalog-key, feature mapping, streamline filename disambiguation, filename parsing, map-completeness invariants |
| [ux.test.ts](unit/ux.test.ts) | `lib/ux` | command palette fuzzy match + ranking, recent-stack dedupe/cap, `isModifierComboMatch` (mod/shift/esc/case), vendor routing, command-id uniqueness + per-command icon key; `githubReleaseTagUrl` (strips leading v) + `EXTERNAL_URLS` release/Nexus links; `matchedIndices` (contiguous span / subsequence / no-match) + `highlightSegments` hit-split |
| [notifications.test.ts](unit/notifications.test.ts) | `lib/notifications` | `makeNotificationEntry` factory: defaults, extras incl. `link` (default null + carried), unique id, ISO timestamp, every NotificationKind (incl. driver/system-driver-update + backup-restored) |
| [launcherLogos.test.ts](unit/launcherLogos.test.ts) | `lib/launcherLogos` | 7 brands present, valid hex bg, non-empty SVG path, order-list integrity |
| [dlssPresets.test.ts](unit/dlssPresets.test.ts) | `lib/dlss` | SR preset / FG-mode / FG-count option tables, preset labels, per-option description + source URL, driver-version gating (≥572.16 DLSS 4, ≥595.97 Dynamic MFG), active-override detection |
| [anticheat.test.ts](unit/anticheat.test.ts) | `lib/anticheat` | detection flag (type-guard), joined names, dataset status note, ban-risk warning copy names the anti-cheats |
| [catalogReleases.test.ts](unit/catalogReleases.test.ts) | `lib/catalogReleases` | `mergeFamilyReleases` across feature families: dedupe by version+sha, newest-first sort, distinct same-version files kept, empty input |
| [driversActions.test.ts](unit/driversActions.test.ts) | `lib/drivers` | per-vendor action routing: `canInstall` (direct download only), `isOpenPageOnly` (AMD: update + no download + has page), mutual exclusivity, `driverPageUrl` notes→PDF→null fallback, `vendorHelpUrl` per-vendor finder + Windows fallback |
| [brands.test.ts](unit/brands.test.ts) | `lib/brands` | `BRANDS` registry (4 core vendors + realtek/dell/msi/asus/gigabyte/qualcomm/logitech/razer; non-empty label + real svg path + viewBox + `--vendor-*`/token accent), `resolveBrandKey` (messy WUA provider strings → key; AMD/ATI, Nahimic/A-Volute→realtek, MSI/Micro-Star, ROG→asus, AORUS→gigabyte, Snapdragon→qualcomm; unknown/empty/null→null), `brandLabel` (clean label or echo raw), `brandFor` |
| [designTokens.test.ts](unit/designTokens.test.ts) | `styles/global.css` | source-level token contract: 8pt `--space-1..8` scale step values, density tokens, the shared tactile (`.hover-lift`/`.press`) + `.glass-dialog`/`.dialog-close` utilities present and centralized |

## Frontend — integration (`tests/integration/`)

| File | Surface | Coverage |
|:---|:---|:---|
| [catalogDiff.test.ts](integration/catalogDiff.test.ts) | `stores.diffCatalogLatest` | first-load suppression, changed-only emission, new-family skip, removed-family skip |
| [libraryStatus.test.ts](integration/libraryStatus.test.ts) | `relation` + Library Sort policy | mixed-library status derivation; outdated-first ordering contract (status rank → alpha) |
| [toastStore.test.ts](integration/toastStore.test.ts) | Toast popup data layer | append/kind/message, FIFO stacking, TTL auto-dismiss (fake timers), targeted dismiss, no-op unknown id |
| [driverStatus.test.ts](integration/driverStatus.test.ts) | `lib/drivers` | status label/tone maps, update detection + count, sort order (update→unknown→up_to_date→unsupported) + alpha tie-break + no-mutate |
| [driverInstall.test.ts](integration/driverInstall.test.ts) | `stores.driverInstall` (UI e2e state machine) | shared install store: stray events ignored when idle; **progress survives a view change mid-download (bug-#1 regression guard)**; one-install-at-a-time; cancelled→warning / failed (Intel exit 8)→danger toast + state cleared; AMD empty-URL report never invokes install (open-page path) |
| [systemDriverInstall.test.ts](integration/systemDriverInstall.test.ts) | `stores.systemDriverInstall` (System & Components install state machine) | shared system-driver install store: stray events ignored when idle; progress survives a view change mid-download; one-install-at-a-time (keyed by `update_id`); success→success toast + scan refreshed; reboot hint surfaced; failed→danger toast + state cleared + no rescan |
| [optimisticApply.test.ts](integration/optimisticApply.test.ts) | `stores.optimisticToggle` (reversible apply + quiet-undo) | optimistic state applied immediately + Undo toast shown; Undo reverts and dismisses; backend rejection reverts + danger toast; late failure after an Undo does not double-revert |

## Frontend — component render (`tests/components/`)

DOM-render tests for every modal/popup via `@testing-library/svelte` on happy-dom. Tauri is satisfied
through injected internals (`window.__TAURI_INTERNALS__` + `__TAURI_EVENT_PLUGIN_INTERNALS__`) and a
WAAPI `Element.prototype.animate` stub so Svelte transitions run headless.

| File | Component | Coverage |
|:---|:---|:---|
| [Toast.test.ts](components/Toast.test.ts) | `Toast` | empty render, message + kind class, stacking, dismiss control |
| [ActivityDock.test.ts](components/ActivityDock.test.ts) | `ActivityDock` (Tidal bottom bar) | idle→nothing; single apply→label + determinate fill width; multi-task→"N tasks running" count; indeterminate fill when no fraction; expand→`applyModalOpen` true; ended applies ignored (live work only) |
| [ShortcutOverlay.test.ts](components/ShortcutOverlay.test.ts) | `ShortcutOverlay` | closed→nothing, open dialog + groups + kbd chips, close-button collapses via store |
| [CommandPalette.test.ts](components/CommandPalette.test.ts) | `CommandPalette` (revamp) | closed→nothing; boxed search field + Esc chip + category chips; grouped section heads (Navigate/Action/Settings) with category-tinted per-row icons; shortcut chip (G L) for commands that declare one; live filter + fuzzy-char highlight (`mark.result-hl`); arrow-key nav across group boundaries; rich empty state echoing the query |
| [NotificationsBell.test.ts](components/NotificationsBell.test.ts) | `NotificationsBell` | closed→nothing, empty state, seeded list + count + dismiss + mark-all, per-kind badge tints (incl. driver/system-driver/backup), external link actions (release→GitHub+Nexus, driver→vendor, none otherwise), unread stripe; **mount-location contract** (panel `position:fixed` not absolute, lifted out of the glass TopBar, mounted at the app root — frosted-glass fix) |
| [ApplyProgressModal.test.ts](components/ApplyProgressModal.test.ts) | `ApplyProgressModal` | dialog shell, completed-group title/version/Updated pill, pane-head detail toggle + stat chips + progress, footer Dismiss aura-pill, collapse toggle hide/show |
| [DlssOverridePanel.test.ts](components/DlssOverridePanel.test.ts) | `DlssOverridePanel` | both feature groups + reversible/anti-cheat note, custom dropdowns + checkboxes (no native controls), DLSS 4 driver warning |
| [Checkbox.test.ts](components/Checkbox.test.ts) | `Checkbox` | role=checkbox + label, toggle on click, no-toggle when disabled |
| [Select.test.ts](components/Select.test.ts) | `Select` | selected label on trigger, opens listbox of options on click, marks the chosen option aria-selected |
| [DriverHistoryFlyout.test.ts](components/DriverHistoryFlyout.test.ts) | `DriverHistoryFlyout` | WHQL-toggle honesty: disabled + label switches when 0 betas loaded; enabled + filter hides 3 betas when 47 WHQL + 3 Beta loaded; footer count `47 of 50` |
| [GameDetailDrawer.test.ts](components/GameDetailDrawer.test.ts) | `GameDetailDrawer` = full-page detail VIEW (rebuilt) | source contract: renders in-flow (no `drawer-scrim`, no fixed/overlay, no `role=dialog`/`aria-modal`/`trapFocus`/`matchMedia`); `.detail-back` "Back to Library" → onClose + Escape; compact `.detail-hero` (`.drawer-art` fixed `height:clamp(...)`, no aspect-ratio) + `.drawer-art::before` launcher-accent stripe; summary scrolls (not sticky) + sticky-bottom `.drawer-foot` action bar; `.learn-more` link-btn doctrine. **App-level wiring**: App renders it when `currentView==="library" && drawerGameId` (replaces Library, not an overlay); Library no longer renders it; push-padding side-panel (`data-drawer-open` / `padding-right:var(--drawer-width)`) removed |
| [ContextMenu.test.ts](components/ContextMenu.test.ts) | `ContextMenu` | one `role=menuitem` per action; keyboard nav (Arrow/Home/End wrap); Escape + outside-pointerdown dismiss; inside-pointerdown keeps open; action callback fires; viewport clamp keeps the menu on-screen |
| [BrandMark.test.ts](components/BrandMark.test.ts) | `BrandMark` | known key → inline `svg.brand-glyph` with the brand path + clean label; messy provider string resolves to the right glyph; unknown key → label-only (no empty glyph); `showLabel=false` hides the label; `tone:'color'` tints via `--vendor-*` token, `tone:'mono'` uses `currentColor`; explicit label overrides the resolved one |
| [material.test.ts](components/material.test.ts) | flat-opaque base material doctrine | source-CSS contract: base content-surface tokens are opaque (never translucent rgba) in both dark and light; GameCard/GameListRow surfaces obey the flat material |
| [glassDialogUnification.test.ts](components/glassDialogUnification.test.ts) | unified floating-chrome doctrine | source-CSS contract: every modal/flyout/popover/dropdown carries `.glass-dialog`, closable dialogs use `.dialog-close` (no bespoke close classes), no local `backdrop-filter`/`vendor-stripe` remains, accent routes through `--edge-color` |

## Frontend — contracts (`tests/contracts/`)

Validates the Tauri command boundary (Rust serde struct ↔ TS DTO) against the JSON Schema in
[contracts/](../contracts/). [_schema.ts](contracts/_schema.ts) is a dependency-free recursive validator.

| File | Contract | Coverage |
|:---|:---|:---|
| [driverRelease.test.ts](contracts/driverRelease.test.ts) | `contracts/driver-release.schema.json` | required-field set guard + NVIDIA/AMD/Intel(Arc)/Intel(integrated 31.x) fixtures conform; AMD empty `download_url` permitted (open-page model) with a notes page present |
| [anticheatReport.test.ts](contracts/anticheatReport.test.ts) | `contracts/anticheat-report.schema.json` | required-field set guard + detected/clean fixtures conform; rejects an unknown detection source |
| [bundleConfig.test.ts](contracts/bundleConfig.test.ts) | `src-tauri/tauri.conf.json` bundle block | publisher/homepage/Apache-2.0 copyright present (PE metadata for antivirus-friction reduction in the no-cert release) |
| [i18nKeyParity.test.ts](contracts/i18nKeyParity.test.ts) | `frontend/src/lib/i18n/locales/{en,es}.json` | every statically referenced `$t()`/`translate()` key resolves in BOTH locales (plural-suffix aware), EN↔ES flat key sets identical, reference scan covers >200 keys — kills the raw-key-leak class |

## Backend — Rust (`cargo test --workspace`)

Inline `#[cfg(test)]` per crate and per command module. All green:

| Area | Module | Coverage |
|:---|:---|:---|
| notifications-store | `crates/notifications-store/src/lib.rs` | round-trip (all kinds incl. driver/system-driver-update + backup-restored), `link` column round-trip + legacy-DB `ensure_column` migration (old schema reads `link=None`, new inserts persist), FIFO eviction at 200, concurrency, graceful skip of unknown kinds |
| backup-store | `crates/backup-store/src/lib.rs` | insert/list/delete, folder-name sanitize, path-prefix rewrite |
| dll-catalog | `crates/dll-catalog/tests/download_resilience.rs` | 12 `wiremock` integration tests (shared-cache one-fetch, retry, cancel, byte-precise progress) |
| paths | `src-tauri/src/paths.rs` | layout, legacy migration idempotence |
| settings / ui_prefs | `src-tauri/src/commands/settings.rs` | v2 round-trip, field defaults |
| shell reveal | `src-tauri/src/commands/shell.rs` | `select_arg` quotes only the path, not the `/select` flag (spaced Steam paths) |
| diagnostics | `src-tauri/src/commands/diagnostics.rs` | percent-encode (unreserved/UTF-8), body truncation, log tail, newest-first log discovery |
| system_info | `src-tauri/src/system_info.rs` | DDR label decode, vendor-id map, per-vendor runtime recommendations, `hardware_id` (uppercase 4-hex `VEN_&DEV_`), `dedupe_adapters` (collapses twin/duplicate GPUs, keeps distinct) |
| driver-catalog | `crates/driver-catalog/src/{version,sources/*}.rs` | per-vendor version normalization (edge-case-five) + `four_part_labeled`, registry source selection, NVIDIA pfid match + Ajax parse + release-notes/date parse, Intel DSA `software-configurations.json` newest-client parse, AMD `amdversions.xml` arch-branch parse (`amd_arch` RDNA/Polaris classify) |
| anticheat-detect | `crates/anticheat-detect/src/lib.rs` | filename fingerprint match (case-insensitive, EAC/BattlEye/Vanguard/GameGuard/XIGNCODE3/EA AC/PunkBuster/HoYoverse), `scan_dir` present/absent/empty/missing-path/depth-limit |
| dll-catalog anti-cheat | `crates/dll-catalog/src/lib.rs` | `normalize_name` (strip punctuation/case), `AntiCheatIndex::lookup` appid precedence → normalized-name fallback, `embedded()` snapshot resolves Elden Ring, `merge` overlay precedence, empty `anti_cheat_binaries` serialization omission |
| manifest-builder | `crates/manifest-builder/src/main.rs` | tag version pack; `distill_anticheat` (normalized keys, flexible `storeIds.steam`); `merge_swapper` maps FSR/XeSS families with vendor signature subject; `vendor_subject` per vendor |
| anti-cheat command | `src-tauri/src/commands/anticheat.rs` | `combine` marks scan vs dataset source, scan wins overlap, empty → no detection; `learn_more_url` routes per-game PCGW when `app_id` parses, glossary anti-cheat when AC kind present, glossary DRM otherwise, None when no detection, glossary fallback when appid unparseable |
| nvapi-drs | `crates/nvapi-drs/src/settings.rs` | DLSS override DRS id/value tables (preset A–M, FG Fixed/Dynamic, MFG count), config→DRS-setting mapping, resettable ids |
| DRS round-trip (destructive, opt-in) | `src-tauri/tests/dlss_profile_roundtrip.rs` | `#[cfg(target_os="windows")] #[ignore]` apply→read→reset on the live NVIDIA base profile; ResetGuard restores defaults on panic. Run with `cargo test -p dlssync --test dlss_profile_roundtrip -- --ignored` on a Windows host with NVIDIA driver |
| driver-install | `crates/driver-install/src/{state,download,verify}.rs` + `tests/download.rs` | install state-machine transitions, exit-code→stage map, log-percent parse; signature gate (rejects unsigned, accepts MS-signed system binary); wiremock download (body→file, 5xx retry, pre-cancel) |
| drivers command | `src-tauri/src/commands/drivers.rs` | OS-family build threshold, GpuVendor→DriverVendor map, GpuInfo→DeviceId + installed-version normalization |
| system-drivers (general PC driver engine) | `crates/system-drivers/src/{version,classify,lib,inventory}.rs` + `tests/pipeline.rs` | version parse/zero-padded compare, `extract_version` from WUA title, OLE-date→ISO (Hinnant civil-from-days), **anti-downgrade `is_newer`** (date-dominant, version tie-break, refuse-when-uncomparable); `DeviceClass` taxonomy + `classify` (specific-before-broad); `hwid_core` VEN/DEV·VID/PID extraction + `matches_device`; **`filter_safe_updates`** (drops non-newer, keeps unmatched, annotates target); `group_by_class`; WMI `cim_to_iso` + `device_from_row` mapper; **pipeline** (fake WMI+WUA → inventory→search→filter→group, drops older wifi / keeps newer audio + unmatched bluetooth) + install progress-stage sequence |
