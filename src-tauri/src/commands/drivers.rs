use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::system_info::{self, GpuInfo, GpuVendor, SystemInfo};
use dll_catalog::DownloadProgress;
use driver_catalog::{
    consts, sources::DEFAULT_HISTORY_LIMIT, DeviceClass, DeviceId, DriverRegistry, DriverRelease,
    DriverStatusReport, DriverVendor, DriverVersion, OsFamily, OsTarget, UpdateStatus,
};
use driver_install::state::{classify_exit, InstallStage};
use driver_install::{download_to_file, verify_signature, DownloadOpts};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

const WINDOWS_11_MIN_BUILD: u32 = 22000;

fn map_vendor(vendor: GpuVendor) -> DriverVendor {
    match vendor {
        GpuVendor::Nvidia => DriverVendor::Nvidia,
        GpuVendor::Amd => DriverVendor::Amd,
        GpuVendor::Intel => DriverVendor::Intel,
        GpuVendor::Other => DriverVendor::Other,
    }
}

fn pci_vendor_id(vendor: DriverVendor) -> u16 {
    match vendor {
        DriverVendor::Nvidia => consts::pci::NVIDIA,
        DriverVendor::Amd => consts::pci::AMD,
        DriverVendor::Intel => consts::pci::INTEL,
        _ => 0,
    }
}

fn os_target(info: &SystemInfo) -> OsTarget {
    let build = info.os.build.parse::<u32>().unwrap_or(0);
    let family = if build >= WINDOWS_11_MIN_BUILD {
        OsFamily::Windows11X64
    } else {
        OsFamily::Windows10X64
    };
    OsTarget { family, dch: true }
}

fn device_for(gpu: &GpuInfo) -> (DeviceId, DriverVersion) {
    let vendor = map_vendor(gpu.vendor);
    let device = DeviceId {
        class: DeviceClass::Gpu,
        vendor,
        pci_vendor_id: pci_vendor_id(vendor),
        pci_device_id: 0,
        model: gpu.model.clone(),
    };
    let installed = DriverVersion::from_installed(vendor, &gpu.driver_version);
    (device, installed)
}

pub(crate) async fn ensure_system_info(state: &State<'_, AppState>) -> AppResult<SystemInfo> {
    {
        let guard = state.system_info.read();
        if let Some(info) = guard.as_ref() {
            return Ok(info.clone());
        }
    }
    let collected = tokio::task::spawn_blocking(system_info::collect)
        .await
        .map_err(|e| crate::error::AppError::Other(format!("system_info collect: {e}")))?;
    *state.system_info.write() = Some(collected.clone());
    Ok(collected)
}

#[tauri::command]
pub async fn check_driver_updates(
    state: State<'_, AppState>,
) -> AppResult<Vec<DriverStatusReport>> {
    let info = ensure_system_info(&state).await?;
    let os = os_target(&info);
    let registry = DriverRegistry::with_default_gpu_sources();
    let client = state.http_catalog.clone();
    let mut reports = Vec::with_capacity(info.gpus.len());
    for gpu in &info.gpus {
        let (device, installed) = device_for(gpu);
        let report = match registry
            .resolve(&client, &device, &os, installed.clone())
            .await
        {
            Ok(report) => report,
            Err(error) => {
                tracing::warn!(model = %gpu.model, %error, "driver lookup failed");
                DriverStatusReport {
                    device,
                    installed,
                    latest: None,
                    status: UpdateStatus::Unknown,
                }
            }
        };
        reports.push(report);
    }
    Ok(reports)
}

#[tauri::command]
pub async fn list_driver_history(
    state: State<'_, AppState>,
    model: String,
    vendor: String,
) -> AppResult<Vec<DriverRelease>> {
    let target_vendor = match vendor.to_ascii_lowercase().as_str() {
        "nvidia" => DriverVendor::Nvidia,
        "amd" => DriverVendor::Amd,
        "intel" => DriverVendor::Intel,
        other => {
            return Err(AppError::Other(format!(
                "unsupported driver vendor: {other}"
            )))
        }
    };
    let info = ensure_system_info(&state).await?;
    let os = os_target(&info);
    let gpu = info
        .gpus
        .iter()
        .find(|g| g.model == model && map_vendor(g.vendor) == target_vendor)
        .cloned()
        .ok_or_else(|| AppError::Other(format!("no detected GPU matches model '{model}'")))?;
    let (device, _) = device_for(&gpu);
    let registry = DriverRegistry::with_default_gpu_sources();
    let client = state.http_catalog.clone();
    registry
        .history(&client, &device, &os, DEFAULT_HISTORY_LIMIT)
        .await
        .map_err(|e| AppError::Other(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub stage: InstallStage,
    pub message: String,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    pub stage: InstallStage,
    pub exit_code: i32,
    pub message: String,
}

const DRIVER_INSTALL_EVENT: &str = "driver_install_progress";

#[cfg(windows)]
fn launch_installer(path: &Path) -> Result<i32, String> {
    driver_install::launch::launch_and_wait(path, None).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn launch_installer(_path: &Path) -> Result<i32, String> {
    Err("driver install requires Windows".to_string())
}

/// Derive the on-disk installer filename from the download URL. The result is
/// joined under the driver cache dir, so it must be a single path component:
/// strip any query/fragment, take the last `/` segment, and reject anything
/// carrying a path separator, a drive/ADS colon, or a `..` traversal so a
/// crafted URL cannot escape the cache directory.
fn installer_filename(url: &str) -> String {
    const FALLBACK: &str = "driver-setup.exe";
    let candidate = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or("");
    let safe = candidate.to_ascii_lowercase().ends_with(".exe")
        && !candidate.contains(['/', '\\', ':'])
        && !candidate.contains("..");
    if safe {
        candidate.to_string()
    } else {
        FALLBACK.to_string()
    }
}

fn emit_stage(app: &AppHandle, stage: InstallStage, message: &str, progress: Option<f64>) {
    let _ = app.emit(
        DRIVER_INSTALL_EVENT,
        InstallProgress {
            stage,
            message: message.to_string(),
            progress,
        },
    );
}

#[tauri::command]
pub async fn install_driver(
    app: AppHandle,
    state: State<'_, AppState>,
    vendor: String,
    download_url: String,
) -> AppResult<InstallOutcome> {
    let cache_dir = {
        let guard = state.paths.read();
        guard
            .as_ref()
            .map(|p| p.cache_dir.clone())
            .ok_or_else(|| AppError::Other("app paths not initialized".into()))?
    };
    let dest = cache_dir
        .join("drivers")
        .join(installer_filename(&download_url));
    let client = state.http_downloads.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
    let app_pump = app.clone();
    let pump = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let fraction = progress
                .bytes_total
                .filter(|total| *total > 0)
                .map(|total| progress.bytes_downloaded as f64 / total as f64);
            emit_stage(
                &app_pump,
                InstallStage::Downloading,
                "Downloading driver",
                fraction,
            );
        }
    });

    let opts = DownloadOpts {
        progress_tx: Some(tx),
        ..Default::default()
    };
    let downloaded = download_to_file(&client, &download_url, &dest, opts)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    let _ = pump.await;

    emit_stage(
        &app,
        InstallStage::Verifying,
        "Verifying vendor signature",
        None,
    );
    let verify_path = downloaded.path.clone();
    let verify_vendor = vendor.clone();
    tokio::task::spawn_blocking(move || verify_signature(&verify_path, &verify_vendor))
        .await
        .map_err(|e| AppError::Other(format!("verify task: {e}")))?
        .map_err(|e| AppError::Other(e.to_string()))?;

    emit_stage(
        &app,
        InstallStage::Launching,
        "Launching installer — accept the Windows UAC prompt",
        None,
    );
    emit_stage(
        &app,
        InstallStage::Installing,
        "Vendor installer is running",
        None,
    );
    let launch_path = downloaded.path.clone();
    let exit_code = tokio::task::spawn_blocking(move || launch_installer(&launch_path))
        .await
        .map_err(|e| AppError::Other(format!("launch task: {e}")))?
        .map_err(AppError::Other)?;

    let stage = classify_exit(exit_code);
    let message = match stage {
        InstallStage::Completed => "Driver installed. A reboot may be required.".to_string(),
        InstallStage::Cancelled => "Installation cancelled.".to_string(),
        _ => format!("Installer exited with code {exit_code}."),
    };
    emit_stage(&app, stage, &message, None);
    Ok(InstallOutcome {
        stage,
        exit_code,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_info::OsInfo;

    fn sysinfo_with_build(build: &str) -> SystemInfo {
        SystemInfo {
            os: OsInfo {
                name: "Windows".into(),
                version: "10.0".into(),
                build: build.into(),
                edition: "Windows 11 Pro".into(),
            },
            cpu: Default::default(),
            ram: Default::default(),
            gpus: vec![],
            collected_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn os_target_picks_windows_11_at_build_threshold() {
        assert_eq!(
            os_target(&sysinfo_with_build("22631")).family,
            OsFamily::Windows11X64
        );
        assert_eq!(
            os_target(&sysinfo_with_build("19045")).family,
            OsFamily::Windows10X64
        );
        assert_eq!(
            os_target(&sysinfo_with_build("")).family,
            OsFamily::Windows10X64
        );
    }

    #[test]
    fn map_vendor_maps_each_gpu_vendor() {
        assert_eq!(map_vendor(GpuVendor::Nvidia), DriverVendor::Nvidia);
        assert_eq!(map_vendor(GpuVendor::Amd), DriverVendor::Amd);
        assert_eq!(map_vendor(GpuVendor::Intel), DriverVendor::Intel);
        assert_eq!(map_vendor(GpuVendor::Other), DriverVendor::Other);
    }

    #[test]
    fn device_for_nvidia_normalizes_installed_version() {
        let gpu = GpuInfo {
            vendor: GpuVendor::Nvidia,
            model: "NVIDIA GeForce RTX 4070 Ti SUPER".into(),
            driver_version: "32.0.15.9174".into(),
            vram_bytes: 0,
            recommended_runtimes: vec![],
        };
        let (device, installed) = device_for(&gpu);
        assert_eq!(device.vendor, DriverVendor::Nvidia);
        assert_eq!(device.pci_vendor_id, consts::pci::NVIDIA);
        assert_eq!(installed.display, "591.74");
    }

    #[test]
    fn installer_filename_keeps_a_clean_exe_segment() {
        assert_eq!(
            installer_filename(
                "https://us.download.nvidia.com/Windows/610.47/610.47-desktop-win10-win11-64bit-international-dch-whql.exe"
            ),
            "610.47-desktop-win10-win11-64bit-international-dch-whql.exe"
        );
    }

    #[test]
    fn installer_filename_strips_query_and_fragment() {
        assert_eq!(
            installer_filename("https://host/setup.exe?token=abc"),
            "setup.exe"
        );
        assert_eq!(
            installer_filename("https://host/setup.exe#frag"),
            "setup.exe"
        );
    }

    #[test]
    fn installer_filename_rejects_path_traversal_and_separators() {
        assert_eq!(
            installer_filename("https://host/x/..\\..\\..\\Windows\\System32\\evil.exe"),
            "driver-setup.exe"
        );
        assert_eq!(
            installer_filename("https://host/C:evil.exe"),
            "driver-setup.exe"
        );
        assert_eq!(
            installer_filename("https://host/..%2fevil.exe"),
            "driver-setup.exe"
        );
    }

    #[test]
    fn installer_filename_falls_back_when_not_an_exe() {
        assert_eq!(installer_filename("https://host/page"), "driver-setup.exe");
        assert_eq!(
            installer_filename("https://host/archive.zip"),
            "driver-setup.exe"
        );
    }
}
