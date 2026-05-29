# Code signing & SmartScreen — the honest reality

This document records, with sources, why DLSSync still triggers a Windows
"unknown publisher" prompt, what this release does to reduce the friction
**without any certificate or paid account**, and the two real fixes kept on file
for when we choose to enable them.

## The honest truth

**Without a code-signing certificate, the SmartScreen / "Windows protected your
PC" prompt cannot be fully removed.** Everything else is friction reduction, not
elimination.

- **Self-signed certificates do nothing for SmartScreen.** A self-signed cert is
  treated exactly like no signature — it carries no reputation and no trusted
  chain. (Microsoft Learn, *SmartScreen* + *Authenticode* docs.)
- **There is no stealthy / automatic / "black-hat" way to manufacture a trusted
  signature.** The only mechanisms that produce a *trusted* Authenticode
  signature are (a) a real, identity-validated code-signing certificate from a
  CA, or (b) the Microsoft Store's re-signing of an MSIX. Anything else that
  claims to "auto-generate a trusted signature" or "bypass SmartScreen" is
  fraudulent or malware:
  - The *Fox Tempest* "Malware-Signing-as-a-Service" operation abused Azure
    Trusted Signing with fraudulent identities to sign malware — **Microsoft
    dismantled it (May 2026)**. Using stolen/fraudulent certs is a criminal
    supply-chain compromise, not an option.
  - "SmartScreen bypass" / "FUD crypter" tools are themselves malware. Shipping
    that fingerprint gets DLSSync **classified as malware** by Defender — the
    exact opposite of building user trust, and **irreversible** once the
    reputation is poisoned.

So: we do **not** self-sign, and we do **not** ship any evasion fingerprint.
That would actively harm the project. Instead we minimize friction honestly and
let reputation accrue.

## What this release does (no cert, no account)

1. **A clean, well-formed MSI** (alongside the NSIS installer and the portable
   zip). The MSI is a standard Windows Installer package with a stable
   **UpgradeCode** (`ebeac857-6d46-4dea-aeb1-cc5254ffae31`) so upgrades and
   uninstalls are clean, and it is **smoke-installed in CI** (silent install +
   uninstall) on every release so a broken package never ships.
2. **Pristine PE metadata + app manifest, no packers/obfuscation.** Correct
   version-info, company/product strings, and execution-level/DPI/long-path
   awareness so Defender's *antivirus* heuristics don't false-positive. We never
   pack or obfuscate the binary (packers are a malware signal).
3. **A stable download URL + artifact naming family**, release to release, so
   SmartScreen reputation accrues to a steady hash lineage instead of resetting
   every version. The portable zip is offered as a lower-friction alternative
   (no installer elevation).
4. **Honest first-run guidance** (README + below) for the one-time
   *More info → Run anyway* step, so users aren't scared off while reputation
   builds.

### MSI vs NSIS — which to download

- **NSIS `*-setup.exe` (recommended for most users):** per-user install
  (`currentUser`, no admin elevation), tray integration, and **in-place
  auto-update** via the Tauri updater.
- **MSI `*.msi`:** a standard Windows Installer package for users and IT who
  prefer MSI (Group Policy / `msiexec` deployment). It is per-machine (the MSI
  norm) and does not participate in the in-app auto-updater.
- **Portable `*-portable.zip`:** no installer at all; lowest friction.

> Note on per-user MSI: a per-user MSI requires a hand-authored WiX template
> (`InstallScope=perUser`). We deliberately did **not** hand-roll one this
> release — an untested custom installer template is a worse risk than shipping
> the standard per-machine MSI, and the per-user need is already covered by the
> NSIS installer. Per-user MSI is a documented future option.

## First run — getting past the prompt (one time)

When you launch the installer the first time, Windows SmartScreen may show
*"Windows protected your PC"*. This is expected for any app from a publisher
without an established reputation yet — it is **not** a virus warning.

1. Click **More info**.
2. Click **Run anyway**.

You only do this once per version. You can verify the download first: every
DLSSync binary keeps its original **vendor Authenticode signature** on the
redistributed DLLs, and release artifacts are published only from the tagged CI
build at `github.com/xt0n1-t3ch/DLSSync/releases`.

## The real fixes (deferred, on file)

When we choose to invest, either of these removes the prompt for real:

- **SignPath.io OSS (free for open-source).** An identity-validated
  Authenticode certificate; SmartScreen reputation then builds against a trusted
  publisher. The CI is already wired for it — the `sign-windows` job runs only
  when the `SIGNPATH_ENABLED` repo variable is `true` (see
  [`.github/workflows/SIGNPATH.md`](../.github/workflows/SIGNPATH.md)). It stays
  dormant until enabled.
- **Microsoft Store (MSIX).** The Store re-signs the package with a trusted
  chain and there is no SmartScreen prompt at all; the Store also handles
  updates. Requires a one-time developer account.

Both are intentionally **out of scope for this release** (no account/cert this
time) and documented here so the decision is explicit and reversible.

## Sources

- Microsoft Learn — *Microsoft Defender SmartScreen overview*; *Authenticode*;
  *MSIX app signing*.
- Microsoft Security Blog / MSRC — *Fox Tempest* Trusted-Signing abuse takedown
  (May 2026).
- SignPath.io — *Open-source code signing* program documentation.
