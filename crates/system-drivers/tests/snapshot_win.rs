//! `#[ignore]`d Windows-only integration smoke for the DriverStore snapshot
//! path. Exporting a package is NON-destructive (it copies the package out of
//! the store), so this is safe to run on a real machine; the rollback /
//! restore-point paths mutate the system and stay manual.
//!
//! Run on a Windows host with: `cargo test -p system-drivers --test snapshot_win -- --ignored`.

#![cfg(windows)]

use std::os::windows::process::CommandExt;
use std::process::Command;
use system_drivers::{driver_snapshot, parse_enum_drivers};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[test]
#[ignore = "touches the live DriverStore via pnputil; run manually on Windows"]
fn exports_a_real_driverstore_package() {
    let output = Command::new("pnputil.exe")
        .args(["/enum-drivers"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("run pnputil /enum-drivers");
    let text = String::from_utf8_lossy(&output.stdout);
    let packages = parse_enum_drivers(&text);
    let Some(pkg) = packages.into_iter().find(|p| !p.published_name.is_empty()) else {
        eprintln!("no third-party DriverStore packages present; nothing to export");
        return;
    };

    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("snapshot");
    driver_snapshot::export_driver(&pkg.published_name, &dest)
        .unwrap_or_else(|e| panic!("export {} failed: {e}", pkg.published_name));

    let exported = std::fs::read_dir(&dest)
        .expect("read snapshot dir")
        .filter_map(|e| e.ok())
        .count();
    assert!(
        exported > 0,
        "export produced no files for {}",
        pkg.published_name
    );
}
