<!-- keep-comment: SEO keyword block harvested by GitHub search + LLM indexers (Stripe/Vercel pattern); invisible to README readers.
  Primary keywords: DLSS updater, DLSS swapper, DLSS Frame Generation, DLSS Ray Reconstruction,
  FSR 3, FSR Frame Generation, Intel XeSS, NVIDIA Reflex, NVIDIA Streamline,
  Microsoft DirectStorage, upscaling, frame generation, RTX, GeForce, Radeon, Arc.
  Stack: Rust, Tauri 2, Svelte 5, Windows portable, Apache-2.0.
-->

<p align="center">
  <a href="https://github.com/xt0n1-t3ch/DLSSync">
    <img src="./.github/assets/banner.svg" alt="DLSSync" width="100%"/>
  </a>
</p>

<p align="center">
  Open-source DLSS updater for Windows. Keeps DLSS, FSR and XeSS DLLs in sync with NVIDIA, AMD, Intel and Microsoft upstream releases. Hash-verified, vendor-signed, fully reversible.
</p>

<p align="center">
  <sub><b>New in v1.2:</b> fixes the Intel XeSS apply that was failing halfway through with random network errors, downloads each shared archive only once instead of four times, retries automatically on flaky GitHub release CDN responses, and a rebuilt failure view tells you exactly what broke with one-click <code>Retry failed</code>, <code>Allow unsigned &amp; retry</code>, and <code>Copy report</code>. Tray badge while applies are running. See <a href="CHANGELOG.md#120---2026-05-24">CHANGELOG</a>.</sub>
</p>

<p align="center">
  <a href="https://github.com/xt0n1-t3ch/DLSSync/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/xt0n1-t3ch/DLSSync?style=flat&color=0a0a0a&logo=github&logoColor=white"></a>
  <a href="https://github.com/xt0n1-t3ch/DLSSync/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/xt0n1-t3ch/DLSSync/ci.yml?style=flat&color=0a0a0a&label=ci&logo=githubactions&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-Apache%202.0-0a0a0a?style=flat"></a>
  <a href="https://github.com/xt0n1-t3ch/DLSSync/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/xt0n1-t3ch/DLSSync?style=flat&color=0a0a0a&logo=github&logoColor=white"></a>
  <a href="https://xt0n1.com"><img alt="Author" src="https://img.shields.io/badge/by-xt0n1-0a0a0a?style=flat"></a>
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-0a0a0a?style=flat&logo=rust&logoColor=white">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri%202-0a0a0a?style=flat&logo=tauri&logoColor=white">
  <img alt="Svelte 5" src="https://img.shields.io/badge/Svelte%205-0a0a0a?style=flat&logo=svelte&logoColor=white">
  <img alt="Vite 6" src="https://img.shields.io/badge/Vite%206-0a0a0a?style=flat&logo=vite&logoColor=white">
  <img alt="TypeScript 5" src="https://img.shields.io/badge/TypeScript%205-0a0a0a?style=flat&logo=typescript&logoColor=white">
  <img alt="Windows 10 / 11" src="https://img.shields.io/badge/Windows%2010%20%7C%2011-0a0a0a?style=flat&logo=windows11&logoColor=white">
</p>

<p align="center">
  <a href="#what-is-dlssync">What is DLSSync</a>
  &nbsp;·&nbsp;
  <a href="#features">Features</a>
  &nbsp;·&nbsp;
  <a href="#security">Security</a>
  &nbsp;·&nbsp;
  <a href="#download">Download</a>
  &nbsp;·&nbsp;
  <a href="#faq">FAQ</a>
  &nbsp;·&nbsp;
  <a href="#sponsor">Sponsor</a>
  &nbsp;·&nbsp;
  <a href="#license">License</a>
</p>

---

<h2 id="what-is-dlssync"><img src="./.github/assets/icons/info.svg" width="26" align="center" alt=""/> &nbsp;What is DLSSync</h2>

DLSSync detects every game installed via Steam, Epic Games, GOG Galaxy, Ubisoft Connect, EA Desktop, Xbox / Microsoft Store and Battle.net. It then keeps the following DLL families synchronized with each vendor's latest publisher release.

<table>
  <thead>
    <tr><th align="left">Vendor</th><th align="left">Family</th><th align="left">DLLs</th></tr>
  </thead>
  <tbody>
    <tr>
      <td rowspan="5"><img src="https://cdn.simpleicons.org/nvidia/76b900" height="14" align="center" alt=""/>&nbsp;<b>NVIDIA</b></td>
      <td>DLSS Super Resolution</td>
      <td><code>nvngx_dlss.dll</code></td>
    </tr>
    <tr><td>DLSS Frame Generation</td><td><code>nvngx_dlssg.dll</code>, <code>sl.dlss_g.dll</code></td></tr>
    <tr><td>DLSS Ray Reconstruction</td><td><code>nvngx_dlssd.dll</code>, <code>sl.dlss_d.dll</code></td></tr>
    <tr><td>NVIDIA Streamline</td><td><code>sl.interposer.dll</code>, <code>sl.common.dll</code>, <code>sl.pcl.dll</code>, <code>sl.nis.dll</code></td></tr>
    <tr><td>NVIDIA Reflex</td><td><code>sl.reflex.dll</code></td></tr>
    <tr>
      <td rowspan="3"><img src="https://cdn.simpleicons.org/intel/0071c5" height="14" align="center" alt=""/>&nbsp;<b>Intel</b></td>
      <td>XeSS Super Resolution</td>
      <td><code>libxess.dll</code>, <code>libxess_dx11.dll</code></td>
    </tr>
    <tr><td>XeSS Frame Generation</td><td><code>libxess_fg.dll</code></td></tr>
    <tr><td>XeLL</td><td><code>libxell.dll</code></td></tr>
    <tr>
      <td rowspan="2"><img src="https://cdn.simpleicons.org/amd/ed1c24" height="14" align="center" alt=""/>&nbsp;<b>AMD</b></td>
      <td>FidelityFX Super Resolution</td>
      <td><code>amd_fidelityfx_*.dll</code>, <code>ffx_fsr3upscaler_x64.dll</code></td>
    </tr>
    <tr><td>FSR Frame Generation</td><td><code>ffx_frameinterpolation_x64.dll</code></td></tr>
    <tr>
      <td><img src="https://cdn.simpleicons.org/dotnet/512BD4" height="14" align="center" alt=""/>&nbsp;<b>Microsoft</b></td>
      <td>DirectStorage</td>
      <td><code>dstorage.dll</code>, <code>dstoragecore.dll</code></td>
    </tr>
  </tbody>
</table>

Replacements pass two independent signature checks. A SHA-256 mismatch or an Authenticode publisher mismatch refuses the write. Every replaced DLL goes into a local SQLite snapshot store, so any change reverts in one click.

---

<h2 id="features"><img src="./.github/assets/icons/sparkles.svg" width="26" align="center" alt=""/> &nbsp;Features</h2>

<table>
  <tr>
    <td width="50%" valign="top">
      <h4>Hash-verified DLLs</h4>
      Every DLL is SHA-256 checked against the public CDN-hosted catalog before it lands in your game folder.
    </td>
    <td width="50%" valign="top">
      <h4>Authenticode publisher gate</h4>
      The signer subject is verified against the known NVIDIA, AMD, Intel and Microsoft publisher certificates. The app never re-signs or repackages.
    </td>
  </tr>
  <tr>
    <td valign="top">
      <h4>One-click rollback</h4>
      Every replaced DLL goes into a local SQLite snapshot store. The Backups tab restores any snapshot in a single click.
    </td>
    <td valign="top">
      <h4>Ed25519-signed auto-update</h4>
      The app checks GitHub Releases on a 6 hour cadence. The bottom-left banner downloads, verifies and restarts. Tampered payloads are rejected.
    </td>
  </tr>
  <tr>
    <td valign="top">
      <h4>Under 100 MB idle RAM</h4>
      Close-to-tray plus Windows EcoQoS Efficiency Mode drops idle CPU to about 0 percent. Task Manager shows the green leaf badge.
    </td>
    <td valign="top">
      <h4>Zero telemetry</h4>
      No analytics, no accounts, no phone-home. The only outbound traffic is the GitHub Releases endpoint, the jsDelivr DLL catalog and Steam's public cover-art CDN.
    </td>
  </tr>
  <tr>
    <td valign="top">
      <h4>Per-user install, no admin</h4>
      NSIS installer in <code>currentUser</code> mode. Installs to <code>%LOCALAPPDATA%\DLSSync\</code>. No UAC prompt, no driver, no kernel hook.
    </td>
    <td valign="top">
      <h4>Every Windows launcher</h4>
      Steam, Epic, GOG, Ubisoft, EA, Xbox, Battle.net, plus arbitrary custom folders for portable installs.
    </td>
  </tr>
</table>

---

<h2 id="security"><img src="./.github/assets/icons/shield.svg" width="26" align="center" alt=""/> &nbsp;Security</h2>

The app gates every DLL replacement behind two independent signature checks.

| Layer | Mechanism | Refuses |
|---|---|---|
| Update payload | Ed25519 signature over the NSIS bundle | An update whose signature does not verify against the embedded public key |
| DLL replacement | SHA-256 plus Authenticode publisher subject match against the catalog | A DLL not signed by NVIDIA, AMD, Intel or Microsoft |
| Rollback | Local SQLite snapshot of every replaced file before the write | Nothing. Restore is offline and instant |

The DLL-sync path has no driver, no kernel-mode hook, no in-process injection — it reads and writes DLL files inside the game's own install directory. Two opt-in features reach beyond that path and are documented separately: the GPU driver updater downloads and launches the vendor's own signed installer (which self-elevates through UAC; DLSSync never elevates itself — see [docs/drivers.md](docs/drivers.md)), and the DLSS preset / frame-generation overrides write a reversible NVIDIA driver application profile through NVAPI, the same mechanism the NVIDIA app uses, not injection (see [docs/dlss-overrides.md](docs/dlss-overrides.md)). Every network call is unauthenticated and visible from `Settings > Detection`.

---

<h2 id="download"><img src="./.github/assets/icons/download.svg" width="26" align="center" alt=""/> &nbsp;Download</h2>

<p align="center">
  <a href="https://github.com/xt0n1-t3ch/DLSSync/releases/latest">
    <img src="./.github/assets/download-button.svg" alt="Download DLSSync v1.2.0 for Windows 10 / 11" width="520"/>
  </a>
</p>

The installer is a per-user NSIS bundle. It installs to `%LOCALAPPDATA%\DLSSync\` without an admin prompt and registers an Add or Remove Programs entry for clean uninstall. Subsequent versions install themselves silently via the in-app update banner.

CLI alternative:

```pwsh
gh release download --repo xt0n1-t3ch/DLSSync --pattern "*setup.exe"
.\DLSSync_*_x64-setup.exe
```

---

<h2 id="build"><img src="./.github/assets/icons/terminal.svg" width="26" align="center" alt=""/> &nbsp;Build from source</h2>

Prerequisites: Rust stable (`rust-toolchain.toml` pins the version), Node 22 LTS, pnpm 9.

```pwsh
git clone https://github.com/xt0n1-t3ch/DLSSync.git
cd DLSSync
pnpm install
pnpm tauri dev
```

Release build:

```pwsh
pnpm tauri build
```

CI validators (run before opening a PR):

```pwsh
pnpm fmt:rust:check
pnpm lint:rust
pnpm --filter dlssync-frontend check
pnpm --filter dlssync-frontend build
cargo check --workspace
```

---

<h2 id="footprint"><img src="./.github/assets/icons/gauge.svg" width="26" align="center" alt=""/> &nbsp;Footprint</h2>

| Metric | Target | Measured |
|---|---|---|
| Installer | under 10 MB | 4.5 MB |
| Cold start | under 500 ms | yes |
| Idle RAM | under 100 MB | yes |
| Idle CPU minimized | about 0 percent | yes (EcoQoS active) |

---

<h2 id="roadmap"><img src="./.github/assets/icons/map.svg" width="26" align="center" alt=""/> &nbsp;Roadmap</h2>

- [x] v1.0: Windows portable, NSIS installer, auto-update banner, tray, EcoQoS Efficiency Mode, all 7 launchers, hash and Authenticode gates, Apache 2.0.
- [x] v1.2: Apply pipeline hardening — shared per-URL download cache, streaming downloads with retry ladder, per-apply cancellation, failure-centric apply modal, tray inflight badge.
- [x] v1.3: GPU driver updater for NVIDIA, Intel and AMD, plus DLSS preset and frame-generation overrides through the NVIDIA driver profile.
- [ ] v1.4: SignPath OSS Authenticode signing. Removes the SmartScreen warning on first run.
- [ ] v1.5: Per-DLL changelog viewer with diff against the currently installed build.
- [ ] v1.6: Custom catalog sources for community-maintained DLL trees.

---

<h2 id="faq"><img src="./.github/assets/icons/help-circle.svg" width="26" align="center" alt=""/> &nbsp;FAQ</h2>

<details>
<summary><b>How is this different from DLSS Updater or DLSS Swapper?</b></summary>

<br/>

DLSSync writes the new DLL into the game's own folder. It does not symlink, hook the loader or proxy load. The whole project ships as a single signed binary. No Python runtime, no .NET dependency. The hash and Authenticode gates are mandatory by default and configurable in `Settings > Advanced` for development builds. Apache 2.0 and you can read every line in this repository.

</details>

<details>
<summary><b>Does it work with anti-cheat?</b></summary>

<br/>

The app writes a DLL into the game's own install directory. That is the same operation a manual file swap performs. Anti-cheat systems that detect modified game files (Easy Anti-Cheat, BattlEye, Riot Vanguard, Denuvo Anti-Tamper) treat a swapped DLL — and, in online titles, a forced DLSS driver-profile override — as a tampered file, which can lead to a kick or ban. There are confirmed reports of bans after both.

DLSSync detects anti-cheat per game (a local scan of the install folder for known anti-cheat binaries, plus a community dataset bundled into the manifest, matched by Steam app id or name) and shows a warning before any DLL swap or DLSS override. It never blocks the action — it surfaces the risk so the choice is yours. Check the policy of your specific title before applying.

</details>

<details>
<summary><b>Does the app phone home?</b></summary>

<br/>

The only outbound traffic is:

- `api.github.com` for the release update check, capped at one request every 6 hours.
- `cdn.jsdelivr.net` for the DLL catalog manifest.
- `cdn.cloudflare.steamstatic.com` and `cdn2.steamgriddb.com` for game cover art, only if the art is not already cached locally.
- For the GPU driver updater, only when you open the Drivers tab or install a driver: `gfwsl.geforce.com` and `raw.githubusercontent.com/ZenitH-AT/nvidia-data` (NVIDIA), `dsadata.intel.com` (Intel), and the AMD driver host (AMD).

Every request is unauthenticated. No hardware identifier, install list or other identifying information is sent.

</details>

<details>
<summary><b>Why Windows only?</b></summary>

<br/>

Linux support is coming soon as more testing is needed.

</details>

<details>
<summary><b>How do I roll back if an update breaks a game?</b></summary>

<br/>

Open the Backups tab. Every DLL the app has replaced is listed with the timestamp, original version, SHA-256 and a Restore button. Snapshots live at `%USERPROFILE%\DLSSync\Backups\` as plain files and you can copy them out manually.

</details>

<details>
<summary><b>Can I pin a specific DLL version?</b></summary>

<br/>

Yes. In the game detail drawer, every DLL family has a version picker covering every release tracked in the catalog, including historical and experimental builds. Pinned versions are stored in `settings.json` and survive rescans.

</details>

<details>
<summary><b>Does it touch DRM, Denuvo or anti-cheat binaries?</b></summary>

<br/>

No. The app reads and writes DLL files inside the game directory. It never patches executables, never touches DRM binaries, never alters anti-cheat files.

</details>

---

<h2 id="contributing"><img src="./.github/assets/icons/git-pull-request.svg" width="26" align="center" alt=""/> &nbsp;Contributing</h2>

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Branch off `main`, keep commits focused, run the validator chain before opening a PR.

---

<h2 id="author"><img src="./.github/assets/icons/user.svg" width="26" align="center" alt=""/> &nbsp;Author</h2>

<p>
  <a href="https://github.com/xt0n1-t3ch">
    <img src="https://cdn.simpleicons.org/github/ffffff" height="18" align="center" alt=""/>
    &nbsp;github.com/xt0n1-t3ch
  </a>
  &nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://discord.com/users/211189703641268224">
    <img src="https://cdn.simpleicons.org/discord/ffffff" height="18" align="center" alt=""/>
    &nbsp;Discord
  </a>
  &nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://xt0n1.com">
    <img src="./.github/assets/icons/globe.svg" height="18" align="center" alt=""/>
    &nbsp;xt0n1.com
  </a>
</p>

If DLSSync saved you a manual DLL swap, a star on the repository helps other gamers find it.

---

<h2 id="sponsor">Sponsor</h2>

DLSSync is built and maintained on free time. Zero telemetry, no paid tier, no upsell. If the app saves you time or you want it to keep tracking new releases, a sponsorship covers the manifest CI, the auto-update signing, and the hours that keep the catalog fresh.

<p>
  <a href="https://ko-fi.com/xt0n1"><img alt="Ko-fi" src="https://img.shields.io/badge/Ko--fi-ff5e5b?style=flat&logo=kofi&logoColor=white"></a>
  <a href="https://github.com/sponsors/xt0n1-t3ch"><img alt="GitHub Sponsors" src="https://img.shields.io/badge/GitHub%20Sponsors-db61a2?style=flat&logo=githubsponsors&logoColor=white"></a>
  <a href="https://www.paypal.me/xt0n1"><img alt="PayPal" src="https://img.shields.io/badge/PayPal-003087?style=flat&logo=paypal&logoColor=white"></a>
</p>

---

<h2 id="license"><img src="./.github/assets/icons/scale.svg" width="26" align="center" alt=""/> &nbsp;License</h2>

Apache 2.0. See [`LICENSE`](LICENSE) and the attribution in [`NOTICE`](NOTICE).

DLSSync is an independent open-source project. It is not endorsed by, sponsored by or affiliated with NVIDIA, Intel, AMD or Microsoft. DLSS, NVIDIA, GeForce, RTX, Reflex and Streamline are trademarks of NVIDIA Corporation. XeSS, Xe and Arc are trademarks of Intel Corporation. FidelityFX, FSR and Radeon are trademarks of Advanced Micro Devices, Inc. DirectStorage, DirectX and Windows are trademarks of Microsoft Corporation. Every redistributed vendor DLL retains its original Authenticode signature.
