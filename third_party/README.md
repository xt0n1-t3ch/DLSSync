# Third-party patches

## `tauri-winrt-notification`

DLSSync vendors `tauri-winrt-notification 0.7.2` from the upstream Tauri
repository because `notify-rust 4.17.0` requires the `0.7` release line.
Upstream `0.7.2` pins `quick-xml 0.37`, which is affected by
`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195`.

The local patch changes only that dependency to `quick-xml 0.41`. The crate's
source and upstream MIT/Apache-2.0 license files are preserved. Remove the
`[patch.crates-io]` entry and this directory once `notify-rust` accepts
`tauri-winrt-notification 0.8` or a patched `0.7` release.
