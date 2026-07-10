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

## Status (2026-07-10)

- `dlss_swapper`: ingests DLSS SR, Frame Generation, and Ray Reconstruction history.
- `streamline`: ingests production runtime plug-ins from NVIDIA's release archives and rejects development/build-artifact copies when a production entry exists.
- `xess`: ingests XeSS SR, XeSS DX11, XeSS Frame Generation, and XeLL from Intel release archives.
- `fsr`: ingests FidelityFX upscaler, frame generation, loader, Vulkan, and denoiser families from AMD release archives.
- `reflex`: ingests the NVIDIA Reflex release source when published independently of Streamline.
- `directstorage`: ingests `dstorage.dll` and `dstoragecore.dll` directly from Microsoft's NuGet package.
- `anticheat`: emits the normalized app-id/name snapshot consumed by local anti-cheat detection.

## Authenticity

The builder downloads vendor release archives, selects the canonical runtime
entry, computes SHA-256, records the exact archive URL and entry path, and stores
the expected publisher for each vendor. DLSSync then performs the actual
Authenticode verification at download/apply time; catalog metadata alone is not
treated as proof of a valid signature.

The published catalog itself is covered by the repository's detached Ed25519
signature. Runtime acceptance therefore requires both a trusted catalog and the
downloaded file's matching hash and Authenticode publisher.

## Output schema

See `dll_catalog::Catalog`. Schema bumped to `schema_version: 2` to introduce
the extended `Release` fields (`signature_subject`, `signature_signed_at`,
`is_dev`, `channel`, `min_driver`). Existing consumers should treat unknown
fields as optional.
