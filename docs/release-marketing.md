# Release marketing and discovery

DLSSync should be discoverable without looking like keyword spam. The public promise is:

> One trusted Windows app that updates DLSS, FSR, XeSS, Streamline, DirectStorage, GPU drivers, and Windows device drivers with vendor signatures, hash checks, backups, and rollback.

## GitHub README

- Lead with the minimalist banner: `.github/assets/nexus/banner-2560x720.png`.
- Keep the first paragraph under 90 words and include the real search phrases users type: DLSS updater, FSR updater, XeSS updater, GPU driver updater, DLSS swapper alternative.
- Keep the visible "Also useful if you searched for" line. Do not hide a keyword block in HTML comments.
- Keep the release chip current with the exact app version and changelog anchor. For v1.6.8, the public hook is mouse side-button back/forward navigation through DLSSync menus and the game-detail drawer, plus the v1.6.7 trust/driver foundation.
- Keep security claims tied to the implementation: signed manifest, SHA-256 catalog hashes, Authenticode publisher gate, official vendor domains, local backup snapshots, one-click rollback, zero telemetry.

## Nexus Mods

Use the Nexus page as the conversion surface, not the full manual. The top description should fit in one scan:

```text
DLSSync updates DLSS, FSR, XeSS, Streamline, DirectStorage, NVIDIA/AMD/Intel GPU drivers, and Windows device drivers from one place. Every DLL or installer is hash-checked, vendor-signed, reversible, and zero-telemetry.
```

Recommended description order:

1. Hero image: `.github/assets/nexus/banner-header-1300x372.png`.
2. One-line value prop.
3. "What it updates" bullets grouped by NVIDIA, AMD, Intel, Microsoft, and Drivers.
4. "Why it is safe" bullets: official sources, signed manifest, hashes, Authenticode, backups, rollback, anti-cheat warnings.
5. "How to use" in five steps: download, scan, review, apply, restore if needed.
6. Changelog excerpt for the current release only.
7. GitHub source link, issue link, and support links.

Nexus BBCode is stricter than old mod pages. Keep formatting simple: headings, bullets, bold labels, links, and images. Avoid nested spoiler-heavy layouts or custom table tricks.

## Asset map

| Surface | Asset |
|:---|:---|
| GitHub hero | `.github/assets/nexus/banner-2560x720.png` |
| Nexus header | `.github/assets/nexus/banner-header-1300x372.png` |
| Social preview | `.github/assets/preview-card-clean.png` |
| Nexus preview card | `.github/assets/nexus/preview-card-clean-600x338.png` |
| Feature gallery | `.github/assets/nexus/gallery/*.png` |

## Growth checklist

- GitHub repo description should include "DLSS, FSR, XeSS, Streamline, GPU drivers, rollback, zero telemetry".
- GitHub topics should include `dlss`, `fsr`, `xess`, `nvidia`, `amd`, `intel`, `gpu-drivers`, `tauri`, `svelte`, `windows`, `pc-gaming`.
- Nexus tags should stay aligned with the current page: Performance Optimization, Modder's Resource, Utilities for Modders, Utilities for Players, Quality of Life.
- Every public release should update README version copy, Nexus "What's new", screenshots/previews when UI changes, and the changelog excerpt.
