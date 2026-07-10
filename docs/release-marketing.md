# Release marketing and discovery

DLSSync should be discoverable without looking like keyword spam. The public promise is:

> One trusted Windows app that updates DLSS, FSR, XeSS, Streamline, DirectStorage, GPU drivers, and Windows device drivers with vendor signatures, hash checks, backups, and rollback.

## GitHub README

- Lead with the minimalist banner: `.github/assets/nexus/banner-2560x720.png`.
- Keep the first paragraph under 90 words and include the real search phrases users type: DLSS updater, FSR updater, XeSS updater, GPU driver updater, DLSS swapper alternative.
- Keep the visible "Also useful if you searched for" line. Do not hide a keyword block in HTML comments.
- Keep the release chip current with the exact app version and changelog anchor. For v1.7.0, lead with review-before-apply trust: signed catalog evidence, exact Update Plans, Operation Journal, CLI parity, portable isolation, and the explicit Nexus-safe catalog refresh.
- Keep security claims tied to the implementation: signed manifest, exact source provenance, per-entry catalog hashes (SHA-256 for vendor-direct assets, MD5 for DLSS Swapper-archived entries), Authenticode publisher gate, local backup snapshots, one-click rollback, zero telemetry. Do not call every asset vendor-direct: historical NVIDIA DLSS and much of the AMD FSR and Intel XeSS back-catalog currently use the labeled DLSS Swapper community archive.

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
6. Nexus behavior: app self-updates and automatic catalog requests are disabled; `Refresh Catalog` is the only action that contacts the signed public upstream.
7. v1.7.0 highlights: Update Plan, Trust Center, Operation Journal, CLI, portable mode, and eight locales.
8. GitHub source, signed manifest, issue, and support links.

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

- GitHub repo description should include "DLSS, FSR, XeSS, signed catalog, GPU drivers, rollback, CLI, zero telemetry".
- GitHub topics should include `dlss`, `fsr`, `xess`, `directstorage`, `nvidia`, `amd`, `intel`, `gpu-drivers`, `signed-catalog`, `tauri`, `svelte`, `windows`, `pc-gaming`.
- Discovery copy may mention "RenderPilot alternative" only where the surrounding comparison is factual, dated, and linked to the generated competitive registry.
- Nexus tags should stay aligned with the current page: Performance Optimization, Modder's Resource, Utilities for Modders, Utilities for Players, Quality of Life.
- Every public release should update README version copy, Nexus "What's new", screenshots/previews when UI changes, and the changelog excerpt.
