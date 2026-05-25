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

## Frontend — integration (`tests/integration/`)

| File | Surface | Coverage |
|:---|:---|:---|
| [catalogDiff.test.ts](integration/catalogDiff.test.ts) | `stores.diffCatalogLatest` | first-load suppression, changed-only emission, new-family skip, removed-family skip |
| [libraryStatus.test.ts](integration/libraryStatus.test.ts) | `relation` + Library Sort policy | mixed-library status derivation; outdated-first ordering contract (status rank → alpha) |
| [toastStore.test.ts](integration/toastStore.test.ts) | Toast popup data layer | append/kind/message, FIFO stacking, TTL auto-dismiss (fake timers), targeted dismiss, no-op unknown id |

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
