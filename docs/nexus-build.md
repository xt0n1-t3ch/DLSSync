# Nexus Mods build

DLSSync has a separate Nexus Mods distribution channel because Nexus Mods does not allow hosted files to auto-update the application itself.

The standard DLSSync build is unchanged. The Nexus build only strips the app self-updater layer:

- no automatic polling for application releases;
- no Tauri updater plugin registration when built with the `nexus` feature;
- no updater endpoint in the generated Nexus Tauri config;
- no `updater:default` permission in the generated Nexus capability file;
- the About and Settings surfaces state that app updates are manual for Nexus compliance;
- the signed DLL catalog URL is pinned to the embedded fallback manifest commit for this build, so DLL download destinations cannot change without a source change and a new application release.

The DLL/FSR/XeSS/Streamline/DirectStorage sync core remains the same: downloads still go through the signed catalog, expected SHA-256 checks, vendor signature checks where applicable, staging, backups, and rollback. For Nexus, the catalog URL is immutable for the release channel instead of tracking `@main`.

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
cargo check -p dlssync --features nexus
```

Expected result:

- `pnpm run check:nexus` passes;
- frontend Nexus build has no app-updater import or `latest.json` endpoint;
- Rust Nexus build compiles with the updater plugin registration disabled;
- `dll-catalog` under `--features nexus` proves the manifest URL is pinned to `FALLBACK_MANIFEST_COMMIT_SHA` and does not contain `@main`;
- generated Nexus config has `plugins.updater.active = false` and no endpoints;
- generated Nexus capability has no `updater:default` permission.

## Nexus support wording

Use this wording when explaining the release:

> This is a Nexus Mods compliant build. DLSSync's core DLL synchronization remains intact, but application self-update is disabled because Nexus Mods does not allow auto-updating hosted utilities. Users install future DLSSync application versions manually from the Nexus Mods page.

## In-app source links

The Nexus build hides every in-app link to the DLSSync GitHub repository so the package surfaces no path to an auto-updating download or to the source repo. The `showSourceLinks` flag (`frontend/src/lib/distribution.ts`, `false` for the nexus build) gates:

- the About header GitHub, Sponsor, and Report buttons;
- the About footer GitHub profile link and the "Star on GitHub" support action;
- the support nudge "Star" action;
- the apply-failure "Report issue" button (the clipboard "Copy report" stays);
- the live GitHub star-count request (no network call to the GitHub API);
- the share action, which targets the Nexus mod page instead of the repo.

The "check for updates" and changelog actions open the Nexus Mods page. Upstream vendor SDK links (NVIDIA, Intel, AMD, Microsoft) and non-GitHub links (Ko-fi, Discord, the project website, the Nexus page) are unaffected. The signed DLL catalog (manifest) repository is still referenced for transparency — it is the audited DLL source Nexus asked to verify, not an application build.
