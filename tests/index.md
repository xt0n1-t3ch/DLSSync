# DLSSync test suite

Centralized tests for the whole app. Frontend logic/integration runs on Vitest (happy-dom);
backend runs on `cargo test`. Run both before shipping.

## Run

| Layer | Command | What it covers |
|:---|:---|:---|
| Frontend | `pnpm --filter dlssync-frontend test` | Vitest unit + integration (`tests/unit`, `tests/integration`) |
| Frontend (watch) | `pnpm --filter dlssync-frontend test:watch` | Same, watch mode |
| Backend | `cargo test --workspace` (from `src-tauri/`) | Every Rust crate + Tauri command surface |
| Types | `pnpm --filter dlssync-frontend check` | svelte-check, 0 errors / 0 warnings |

Vitest config: [frontend/vitest.config.ts](../frontend/vitest.config.ts). Tauri APIs are mocked in
[tests/setup.ts](setup.ts) so store/logic modules import cleanly outside a WebView.

## Frontend — unit (`tests/unit/`)

| File | Module under test | Coverage |
|:---|:---|:---|
| [formatHuman.test.ts](unit/formatHuman.test.ts) | `lib/formatHuman` | byte/speed/eta/duration/elapsed/percent formatting; null·NaN·Infinity·negative·boundary tiers |
| [relation.test.ts](unit/relation.test.ts) | `lib/relation` | version compare, sha short-circuit, vendor-sha match, target resolution, `gameStatusFromRecords` across outdated/up-to-date/no-dlls/scan-failed/disabled |
| [applyErrorClass.test.ts](unit/applyErrorClass.test.ts) | `lib/applyErrorClass` | all 9 error classes, precedence (cancelled→network→signature…), case-insensitivity, action routing, label/tone tables |
| [labels.test.ts](unit/labels.test.ts) | `lib/labels` | family→vendor/group/catalog-key, feature mapping, streamline filename disambiguation, filename parsing, map-completeness invariants |
| [ux.test.ts](unit/ux.test.ts) | `lib/ux` | command palette fuzzy match + ranking, recent-stack dedupe/cap, `isModifierComboMatch` (mod/shift/esc/case), vendor routing, command-id uniqueness |
| [notifications.test.ts](unit/notifications.test.ts) | `lib/notifications` | `makeNotificationEntry` factory: defaults, extras, unique id, ISO timestamp, every NotificationKind |
| [launcherLogos.test.ts](unit/launcherLogos.test.ts) | `lib/launcherLogos` | 7 brands present, valid hex bg, non-empty SVG path, order-list integrity |
| [dlssPresets.test.ts](unit/dlssPresets.test.ts) | `lib/dlss` | SR preset / FG-mode / FG-count option tables, preset labels, per-option description + source URL, driver-version gating (≥572.16 DLSS 4, ≥595.97 Dynamic MFG), active-override detection |
| [anticheat.test.ts](unit/anticheat.test.ts) | `lib/anticheat` | detection flag (type-guard), joined names, dataset status note, ban-risk warning copy names the anti-cheats |
| [catalogReleases.test.ts](unit/catalogReleases.test.ts) | `lib/catalogReleases` | `mergeFamilyReleases` across feature families: dedupe by version+sha, newest-first sort, distinct same-version files kept, empty input |

## Frontend — integration (`tests/integration/`)

| File | Surface | Coverage |
|:---|:---|:---|
| [catalogDiff.test.ts](integration/catalogDiff.test.ts) | `stores.diffCatalogLatest` | first-load suppression, changed-only emission, new-family skip, removed-family skip |
| [libraryStatus.test.ts](integration/libraryStatus.test.ts) | `relation` + Library Sort policy | mixed-library status derivation; outdated-first ordering contract (status rank → alpha) |
| [toastStore.test.ts](integration/toastStore.test.ts) | Toast popup data layer | append/kind/message, FIFO stacking, TTL auto-dismiss (fake timers), targeted dismiss, no-op unknown id |
| [driverStatus.test.ts](integration/driverStatus.test.ts) | `lib/drivers` | status label/tone maps, update detection + count, sort order (update→unknown→up_to_date→unsupported) + alpha tie-break + no-mutate |

## Frontend — component render (`tests/components/`)

DOM-render tests for every modal/popup via `@testing-library/svelte` on happy-dom. Tauri is satisfied
through injected internals (`window.__TAURI_INTERNALS__` + `__TAURI_EVENT_PLUGIN_INTERNALS__`) and a
WAAPI `Element.prototype.animate` stub so Svelte transitions run headless.

| File | Component | Coverage |
|:---|:---|:---|
| [Toast.test.ts](components/Toast.test.ts) | `Toast` | empty render, message + kind class, stacking, dismiss control |
| [ShortcutOverlay.test.ts](components/ShortcutOverlay.test.ts) | `ShortcutOverlay` | closed→nothing, open dialog + groups + kbd chips, close-button collapses via store |
| [CommandPalette.test.ts](components/CommandPalette.test.ts) | `CommandPalette` | closed→nothing, spotlight input + category chips, command list + category tags, live query filtering |
| [NotificationsBell.test.ts](components/NotificationsBell.test.ts) | `NotificationsBell` | closed→nothing, empty state, seeded list + count + dismiss + mark-all, per-kind badge tints, unread stripe |
| [ApplyProgressModal.test.ts](components/ApplyProgressModal.test.ts) | `ApplyProgressModal` | dialog shell, completed-group title/version/Updated pill, pane-head detail toggle + stat chips + progress, footer Dismiss aura-pill, collapse toggle hide/show |
| [DlssOverridePanel.test.ts](components/DlssOverridePanel.test.ts) | `DlssOverridePanel` | both feature groups + reversible/anti-cheat note, custom dropdowns + checkboxes (no native controls), DLSS 4 driver warning |
| [Checkbox.test.ts](components/Checkbox.test.ts) | `Checkbox` | role=checkbox + label, toggle on click, no-toggle when disabled |
| [Select.test.ts](components/Select.test.ts) | `Select` | selected label on trigger, opens listbox of options on click, marks the chosen option aria-selected |
| [DriverHistoryFlyout.test.ts](components/DriverHistoryFlyout.test.ts) | `DriverHistoryFlyout` | WHQL-toggle honesty: disabled + label switches when 0 betas loaded; enabled + filter hides 3 betas when 47 WHQL + 3 Beta loaded; footer count `47 of 50` |
| [GameDetailDrawer.test.ts](components/GameDetailDrawer.test.ts) | `GameDetailDrawer` redesign | source-CSS contract: `.drawer-scrim` has no `backdrop-filter` + has `radial-gradient` + `prefers-reduced-transparency` fallback; `.drawer::before` highlight + `.drawer-art::before` launcher-accent stripe; `.learn-more` aligned with link-btn doctrine; widened section rhythm (warning 20 / summary 16 / advanced 16) |

## Frontend — contracts (`tests/contracts/`)

Validates the Tauri command boundary (Rust serde struct ↔ TS DTO) against the JSON Schema in
[contracts/](../contracts/). [_schema.ts](contracts/_schema.ts) is a dependency-free recursive validator.

| File | Contract | Coverage |
|:---|:---|:---|
| [driverRelease.test.ts](contracts/driverRelease.test.ts) | `contracts/driver-release.schema.json` | required-field set guard + NVIDIA/AMD/Intel release fixtures conform (version, changelog, display_version, release-notes URL) |
| [anticheatReport.test.ts](contracts/anticheatReport.test.ts) | `contracts/anticheat-report.schema.json` | required-field set guard + detected/clean fixtures conform; rejects an unknown detection source |

## Backend — Rust (`cargo test --workspace`)

Inline `#[cfg(test)]` per crate and per command module. All green:

| Area | Module | Coverage |
|:---|:---|:---|
| notifications-store | `crates/notifications-store/src/lib.rs` | round-trip, FIFO eviction at 200, concurrency, graceful skip of unknown kinds |
| backup-store | `crates/backup-store/src/lib.rs` | insert/list/delete, folder-name sanitize, path-prefix rewrite |
| dll-catalog | `crates/dll-catalog/tests/download_resilience.rs` | 12 `wiremock` integration tests (shared-cache one-fetch, retry, cancel, byte-precise progress) |
| paths | `src-tauri/src/paths.rs` | layout, legacy migration idempotence |
| settings / ui_prefs | `src-tauri/src/commands/settings.rs` | v2 round-trip, field defaults |
| shell reveal | `src-tauri/src/commands/shell.rs` | `select_arg` quotes only the path, not the `/select` flag (spaced Steam paths) |
| diagnostics | `src-tauri/src/commands/diagnostics.rs` | percent-encode (unreserved/UTF-8), body truncation, log tail, newest-first log discovery |
| system_info | `src-tauri/src/system_info.rs` | DDR label decode, vendor-id map, per-vendor runtime recommendations |
| driver-catalog | `crates/driver-catalog/src/{version,sources/*}.rs` | per-vendor version normalization (edge-case-five) + `four_part_labeled`, registry source selection, NVIDIA pfid match + Ajax parse + release-notes/date parse, Intel DSA `software-configurations.json` newest-client parse, AMD `amdversions.xml` arch-branch parse (`amd_arch` RDNA/Polaris classify) |
| anticheat-detect | `crates/anticheat-detect/src/lib.rs` | filename fingerprint match (case-insensitive, EAC/BattlEye/Vanguard/GameGuard/XIGNCODE3/EA AC/PunkBuster/HoYoverse), `scan_dir` present/absent/empty/missing-path/depth-limit |
| dll-catalog anti-cheat | `crates/dll-catalog/src/lib.rs` | `normalize_name` (strip punctuation/case), `AntiCheatIndex::lookup` appid precedence → normalized-name fallback, `embedded()` snapshot resolves Elden Ring, `merge` overlay precedence |
| manifest-builder | `crates/manifest-builder/src/main.rs` | tag version pack; `distill_anticheat` (normalized keys, flexible `storeIds.steam`); `merge_swapper` maps FSR/XeSS families with vendor signature subject; `vendor_subject` per vendor |
| anti-cheat command | `src-tauri/src/commands/anticheat.rs` | `combine` marks scan vs dataset source, scan wins overlap, empty → no detection; `learn_more_url` routes per-game PCGW when `app_id` parses, glossary anti-cheat when AC kind present, glossary DRM otherwise, None when no detection, glossary fallback when appid unparseable |
| nvapi-drs | `crates/nvapi-drs/src/settings.rs` | DLSS override DRS id/value tables (preset A–M, FG Fixed/Dynamic, MFG count), config→DRS-setting mapping, resettable ids |
| DRS round-trip (destructive, opt-in) | `src-tauri/tests/dlss_profile_roundtrip.rs` | `#[cfg(target_os="windows")] #[ignore]` apply→read→reset on the live NVIDIA base profile; ResetGuard restores defaults on panic. Run with `cargo test -p dlssync --test dlss_profile_roundtrip -- --ignored` on a Windows host with NVIDIA driver |
| driver-install | `crates/driver-install/src/{state,download,verify}.rs` + `tests/download.rs` | install state-machine transitions, exit-code→stage map, log-percent parse; signature gate (rejects unsigned, accepts MS-signed system binary); wiremock download (body→file, 5xx retry, pre-cancel) |
| drivers command | `src-tauri/src/commands/drivers.rs` | OS-family build threshold, GpuVendor→DriverVendor map, GpuInfo→DeviceId + installed-version normalization |
