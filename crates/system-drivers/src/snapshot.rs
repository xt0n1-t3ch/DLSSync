//! DriverStore snapshot + rollback for System & Components driver updates.
//!
//! Before installing a non-GPU driver, the elevated install child snapshots the
//! currently-installed driver package from the local DriverStore via
//! `pnputil /export-driver` and lays down a System Restore checkpoint. A bad
//! update can then be rolled back either by re-installing the exported package
//! (`pnputil /add-driver /install`) or via Windows System Restore. Only
//! vendor-signed packages already on the machine are ever touched — no scraping,
//! no unsigned binaries.
//!
//! The `pnputil` argument vectors are pure (unit-tested); the process launch,
//! file IO and `SRSetRestorePointW` FFI live behind `#[cfg(windows)]` and are
//! exercised by the `#[ignore]`d admin integration test plus manual validation.

/// True when `name` is a DriverStore published package name (`oemNN.inf`), the
/// only handle `pnputil /export-driver` accepts. Guards the export against being
/// handed an arbitrary path or a vendor INF name.
pub fn is_published_oem_inf(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("oem") else {
        return false;
    };
    let Some(digits) = rest.strip_suffix(".inf") else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// `pnputil` arguments to export one DriverStore package (`oemNN.inf`) into the
/// `dest` directory.
pub fn export_driver_args(published_name: &str, dest: &str) -> Vec<String> {
    vec![
        "/export-driver".to_string(),
        published_name.to_string(),
        dest.to_string(),
    ]
}

/// `pnputil` arguments to (re)install every INF found under an exported snapshot
/// — `/subdirs` recurses, `/install` forces the package onto matching devices so
/// an explicit user rollback wins even when Windows considers the live driver
/// newer.
pub fn add_driver_install_args(inf_glob: &str) -> Vec<String> {
    vec![
        "/add-driver".to_string(),
        inf_glob.to_string(),
        "/subdirs".to_string(),
        "/install".to_string(),
    ]
}

/// The `*.inf` spec `pnputil /add-driver … /subdirs` expects to find every INF
/// inside a recursively-exported snapshot directory.
pub fn restore_inf_glob(export_dir: &str) -> String {
    let trimmed = export_dir.trim_end_matches(['\\', '/']);
    format!("{trimmed}\\*.inf")
}

#[cfg(windows)]
pub mod win {
    use super::{
        add_driver_install_args, export_driver_args, is_published_oem_inf, restore_inf_glob,
    };
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn run_pnputil(args: &[String]) -> Result<String, String> {
        let output = Command::new("pnputil.exe")
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("failed to launch pnputil: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            return Ok(stdout);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "pnputil exited with code {}: {} {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        ))
    }

    /// Snapshot the installed driver package `published_name` (`oemNN.inf`) into
    /// `dest`, creating `dest` first. Requires Administrator (run inside the
    /// elevated install child).
    pub fn export_driver(published_name: &str, dest: &Path) -> Result<(), String> {
        if !is_published_oem_inf(published_name) {
            return Err(format!(
                "refusing to export a non-DriverStore name: {published_name}"
            ));
        }
        std::fs::create_dir_all(dest).map_err(|e| format!("create snapshot dir: {e}"))?;
        let dest_str = dest.to_string_lossy();
        run_pnputil(&export_driver_args(published_name, &dest_str)).map(|_| ())
    }

    /// Roll back by re-installing every INF in a previously-exported snapshot
    /// directory. Requires Administrator.
    pub fn restore_driver(export_dir: &Path) -> Result<(), String> {
        if !export_dir.is_dir() {
            return Err(format!(
                "snapshot directory not found: {}",
                export_dir.display()
            ));
        }
        let glob = restore_inf_glob(&export_dir.to_string_lossy());
        run_pnputil(&add_driver_install_args(&glob)).map(|_| ())
    }

    /// Lay down a `DEVICE_DRIVER_INSTALL` System Restore checkpoint named
    /// `description`. Returns `Ok(true)` when a checkpoint was created,
    /// `Ok(false)` when System Restore is disabled or throttled (the 24h
    /// `SystemRestorePointCreationFrequency` window) — never an error, because the
    /// exported package snapshot is the dependable rollback and the checkpoint is
    /// only a secondary safety net. Requires Administrator.
    pub fn create_restore_point(description: &str) -> Result<bool, String> {
        use windows::Win32::System::Restore::{
            SRSetRestorePointW, BEGIN_SYSTEM_CHANGE, DEVICE_DRIVER_INSTALL, END_SYSTEM_CHANGE,
            RESTOREPOINTINFOW, STATEMGRSTATUS,
        };

        let mut desc = [0u16; 256];
        for (slot, ch) in desc.iter_mut().zip(description.encode_utf16().take(255)) {
            *slot = ch;
        }

        unsafe {
            let mut info = RESTOREPOINTINFOW {
                dwEventType: BEGIN_SYSTEM_CHANGE,
                dwRestorePtType: DEVICE_DRIVER_INSTALL,
                llSequenceNumber: 0,
                szDescription: desc,
            };
            let mut status = STATEMGRSTATUS::default();
            if !SRSetRestorePointW(&info, &mut status).as_bool() {
                return Ok(false);
            }
            let sequence = status.llSequenceNumber;
            info.dwEventType = END_SYSTEM_CHANGE;
            info.llSequenceNumber = sequence;
            let _ = SRSetRestorePointW(&info, &mut status);
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_driverstore_published_names_only() {
        assert!(is_published_oem_inf("oem47.inf"));
        assert!(is_published_oem_inf("OEM0.INF"));
        assert!(!is_published_oem_inf("nahimicv3.inf"));
        assert!(!is_published_oem_inf("oem.inf"));
        assert!(!is_published_oem_inf("oem47"));
        assert!(!is_published_oem_inf(r"C:\Windows\INF\oem47.inf"));
        assert!(!is_published_oem_inf(""));
    }

    #[test]
    fn export_args_are_positional() {
        assert_eq!(
            export_driver_args("oem47.inf", r"C:\snap\oem47"),
            vec!["/export-driver", "oem47.inf", r"C:\snap\oem47"]
        );
    }

    #[test]
    fn install_args_force_and_recurse() {
        assert_eq!(
            add_driver_install_args(r"C:\snap\oem47\*.inf"),
            vec![
                "/add-driver",
                r"C:\snap\oem47\*.inf",
                "/subdirs",
                "/install"
            ]
        );
    }

    #[test]
    fn restore_glob_appends_inf_spec_and_normalizes_separator() {
        assert_eq!(restore_inf_glob(r"C:\snap\oem47"), r"C:\snap\oem47\*.inf");
        assert_eq!(restore_inf_glob(r"C:\snap\oem47\"), r"C:\snap\oem47\*.inf");
        assert_eq!(restore_inf_glob("C:/snap/oem47/"), r"C:/snap/oem47\*.inf");
    }
}
