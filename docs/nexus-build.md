# Nexus Mods build

DLSSync has a separate Nexus Mods distribution channel because Nexus Mods does not allow hosted files to auto-update the application itself.

The standard DLSSync build is unchanged. The Nexus build only strips the app self-updater layer:

- no automatic polling for application releases;
- no Tauri updater plugin registration when built with the `nexus` feature;
- no updater endpoint in the generated Nexus Tauri config;
- no `updater:default` permission in the generated Nexus capability file;
- the About and Settings surfaces state that app updates are manual for Nexus compliance;
- opening the app, opening Catalog, restoring focus, and background ticks make zero catalog requests;
- `Refresh Catalog` is the only action allowed to fetch the current signed public manifest.

The DLL/FSR/XeSS/Streamline/DirectStorage sync core remains the same: downloads still go through the signed catalog, expected hash checks, Authenticode publisher enforcement, staging, backups, and rollback. The embedded signed catalog is the offline/default state; a user-triggered refresh may replace it only after Ed25519, schema, non-empty, and anti-downgrade checks pass.

## Build commands

Prepare and verify the Nexus build strip artifacts without packaging:

```powershell
pnpm run check:nexus
```

Build the Nexus package:

```powershell
pnpm run build:nexus
```

`pnpm run build:nexus` temporarily swaps the Tauri config and default capability file with generated Nexus-compliant versions, runs `tauri build --features nexus`, and restores the normal source files afterward.

Generated proof inputs live under `target/nexus/`:

- `target/nexus/tauri.conf.json`
- `target/nexus/default.capability.json`

These files are generated from the normal source config so the release lane cannot drift silently.

## Verification expectations

Before uploading to Nexus Mods, run:

```powershell
pnpm run check:nexus
pnpm --filter dlssync-frontend build:nexus
rg "plugin-updater|latest\\.json|downloadAndInstall|tauri_plugin_updater" frontend/dist target/nexus -S
cargo test -p dll-catalog --features nexus manifest_url_targets_the_signed_manifest_repo
cargo test -p dlssync-application nexus_blocks_automatic_but_allows_manual_catalog_refresh
cargo xtask verify-release --channel nexus
cargo check -p dlssync --features nexus
```

Expected result:

- `pnpm run check:nexus` passes;
- frontend Nexus build has no app-updater import or `latest.json` endpoint;
- Rust Nexus build compiles with the updater plugin registration disabled;
- `dll-catalog` proves the embedded fallback remains pinned and signed;
- `dlssync-application` proves the Nexus policy blocks `automatic` and permits only `manual_user` refresh triggers;
- generated Nexus config has `plugins.updater.active = false` and no endpoints;
- generated Nexus capability has no `updater:default` permission.

## Nexus support wording

Use this wording when explaining the release:

> This is a Nexus Mods compliant build. DLSSync's core DLL synchronization remains intact, but application self-update is disabled because Nexus Mods does not allow auto-updating hosted utilities. Users install future DLSSync application versions manually from the Nexus Mods page.

## In-app source links

The Nexus build exposes the application source and the signed public catalog as transparency links. These links do not install or auto-update the application. The application update action itself remains disabled and routes users to the Nexus Mods page; the Tauri updater plugin, endpoint, permission, and polling code stay absent from the Nexus package.

The Catalog footer states that automatic catalog updates are disabled. Its explicit button contacts the canonical manifest repository only after the user clicks it, and the Trust Center shows the signature result, generation time, refresh method, and pinned public-key fingerprint.
