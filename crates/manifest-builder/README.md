# manifest-builder

Generates `manifest/manifest.json` for the DLSSync app.

## Usage

```pwsh
cargo run -p manifest-builder -- --out manifest/manifest.json
cargo run -p manifest-builder -- --out manifest/manifest.json --sources dlss_swapper
cargo run -p manifest-builder -- --dry-run
```

## Sources

| Source key | Provides | Endpoint |
|:---|:---|:---|
| `dlss_swapper` | DLSS SR / Frame Generation / Ray Reconstruction with `signed_datetime`, MD5, `is_signature_valid`, `file_size`, `download_url` | `https://raw.githubusercontent.com/beeradmoore/dlss-swapper/main/docs/manifest.json` |
| `streamline` | Streamline plug-ins (`sl.interposer`, `sl.dlss`, `sl.dlss_g`, `sl.dlss_d`, `sl.reflex`, `sl.pcl`, `sl.nis`, `sl.common`, `sl.directsr`) | `https://api.github.com/repos/NVIDIA-RTX/Streamline/releases` |
| `xess` | `libxess.dll`, `libxess_fg.dll`, `libxell.dll` | `https://api.github.com/repos/intel/xess/releases` |
| `fsr` | `amd_fidelityfx_*.dll`, `ffx_*.dll` | `https://api.github.com/repos/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/releases` |
| `reflex` | NVIDIA Reflex SDK + `sl.reflex.dll` | `https://api.github.com/repos/NVIDIA-RTX/REFLEX/releases` |
| `directstorage` | `dstorage.dll`, `dstoragecore.dll` | NuGet feed `Microsoft.Direct3D.DirectStorage` |

## Status (2026-05-20)

- `dlss_swapper` ingest: working end-to-end (DLSS SR / FG / RR).
- `streamline` / `xess` / `fsr` / `reflex` / `directstorage`: scaffold present, ingest logic stub (warning logged). Implementations land per FR-040 in `specs/003-version-picker-uiux/spec.md`.

## Authenticity

Each release record carries `signed: bool`. Once GitHub-release ingest is wired
up, the builder will:

1. Download the release zip.
2. Locate each canonical `.dll` inside.
3. Compute SHA-256.
4. Read Authenticode subject CN via `wintrust::WinVerifyTrust` (Windows).
5. Reject the release if the subject does not match the family's expected CN
   (`NVIDIA Corporation`, `Intel Corporation`, `Advanced Micro Devices, Inc.`,
   `Microsoft Corporation`).

## Output schema

See `dll_catalog::Catalog`. Schema bumped to `schema_version: 2` to introduce
the extended `Release` fields (`signature_subject`, `signature_signed_at`,
`is_dev`, `channel`, `min_driver`). Existing consumers should treat unknown
fields as optional.
