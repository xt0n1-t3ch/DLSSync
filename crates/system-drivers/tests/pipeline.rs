//! End-to-end pipeline test against fake WMI/WUA boundaries: inventory →
//! search → anti-downgrade filter → group-by-class, with no live Windows
//! Update service. Proves the orchestration the real commands run.

use system_drivers::{
    dedup_updates, filter_safe_updates, group_by_class, DeviceCatalog, DeviceClass, DriverError,
    DriverUpdate, InstallProgress, InstallReport, InstallStage, SystemDevice, UpdateSource,
};

struct FakeInventory(Vec<SystemDevice>);
impl DeviceCatalog for FakeInventory {
    fn inventory(&self) -> Result<Vec<SystemDevice>, DriverError> {
        Ok(self.0.clone())
    }
}

struct FakeSource(Vec<DriverUpdate>);
impl UpdateSource for FakeSource {
    fn search(&self) -> Result<Vec<DriverUpdate>, DriverError> {
        Ok(self.0.clone())
    }
    fn install(
        &self,
        update_id: &str,
        on_progress: &mut dyn FnMut(InstallProgress),
    ) -> Result<InstallReport, DriverError> {
        on_progress(InstallProgress {
            stage: InstallStage::Downloading,
            message: "downloading".into(),
            fraction: None,
        });
        on_progress(InstallProgress {
            stage: InstallStage::Installing,
            message: "installing".into(),
            fraction: None,
        });
        Ok(InstallReport {
            success: true,
            reboot_required: false,
            result_code: 2,
            message: format!("installed {update_id}"),
        })
    }
}

fn device(name: &str, class: DeviceClass, hwid: &str, ver: &str, date: &str) -> SystemDevice {
    SystemDevice {
        name: name.into(),
        class,
        manufacturer: "Vendor".into(),
        driver_version: Some(ver.into()),
        driver_date: Some(date.into()),
        hardware_id: hwid.into(),
        inf_name: None,
        present: true,
    }
}

fn update(title: &str, class: DeviceClass, hwid: &str, ver: &str, date: &str) -> DriverUpdate {
    DriverUpdate {
        update_id: format!("{title}:1"),
        title: title.into(),
        class,
        provider: "Vendor".into(),
        driver_version: Some(ver.into()),
        driver_date: Some(date.into()),
        hardware_id: Some(hwid.into()),
        size_bytes: 1234,
        target_device: None,
        current_version: None,
        target_inf: None,
        target_hardware_id: None,
        support_url: None,
    }
}

#[test]
fn full_scan_pipeline_filters_and_groups() {
    let inventory = FakeInventory(vec![
        device(
            "Realtek High Definition Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.9461.1",
            "2025-08-01",
        ),
        device(
            "Intel(R) Wi-Fi 6 AX201",
            DeviceClass::Network,
            r"PCI\VEN_8086&DEV_A0F0&SUBSYS_X\x",
            "22.250.0.4",
            "2026-03-01",
        ),
    ]);

    let updates = vec![
        update(
            "Realtek - MEDIA - 6.0.9600.1",
            DeviceClass::Audio,
            r"hdaudio\func_01&ven_10ec&dev_0256",
            "6.0.9600.1",
            "2026-04-10",
        ),
        update(
            "Intel - Net - 22.100.0.1",
            DeviceClass::Network,
            r"pci\ven_8086&dev_a0f0",
            "22.100.0.1",
            "2025-12-01",
        ),
        update(
            "Intel - Bluetooth - 23.10.0.2",
            DeviceClass::Bluetooth,
            r"USB\VID_8087&PID_0026",
            "23.10.0.2",
            "2026-05-01",
        ),
    ];

    let devices = inventory.inventory().unwrap();
    let found = FakeSource(updates).search().unwrap();
    let safe = filter_safe_updates(&devices, found);

    assert_eq!(safe.len(), 2);
    let audio = safe.iter().find(|u| u.class == DeviceClass::Audio).unwrap();
    assert_eq!(
        audio.target_device.as_deref(),
        Some("Realtek High Definition Audio")
    );
    let bt = safe
        .iter()
        .find(|u| u.class == DeviceClass::Bluetooth)
        .unwrap();
    assert_eq!(bt.target_device, None);
    assert!(!safe.iter().any(|u| u.class == DeviceClass::Network));

    let groups = group_by_class(safe);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].class, DeviceClass::Audio);
    assert_eq!(groups[1].class, DeviceClass::Bluetooth);
}

#[test]
fn hybrid_gpu_excluded_components_kept_and_deduped() {
    let inventory = FakeInventory(vec![
        device(
            "NVIDIA GeForce RTX 4070",
            DeviceClass::Display,
            r"PCI\VEN_10DE&DEV_2D05&SUBSYS_X\x",
            "560.0.0.0",
            "2026-01-01",
        ),
        device(
            "Intel(R) Iris Xe Graphics",
            DeviceClass::Display,
            r"PCI\VEN_8086&DEV_9A49\x",
            "31.0.101.2141",
            "2026-01-01",
        ),
        device(
            "Realtek High Definition Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.9461.1",
            "2025-08-01",
        ),
    ]);

    let updates = vec![
        update(
            "NVIDIA - Display - 565.0.0.0",
            DeviceClass::Display,
            r"pci\ven_10de&dev_2d05",
            "565.0.0.0",
            "2026-05-01",
        ),
        update(
            "Intel - Display - 31.0.101.8000",
            DeviceClass::Display,
            r"pci\ven_8086&dev_9a49",
            "31.0.101.8000",
            "2026-05-01",
        ),
        update(
            "Realtek - MEDIA - 6.0.9600.1",
            DeviceClass::Audio,
            r"hdaudio\func_01&ven_10ec&dev_0256",
            "6.0.9600.1",
            "2026-04-10",
        ),
        update(
            "Realtek - MEDIA - 6.0.9600.1 (mirror)",
            DeviceClass::Audio,
            r"hdaudio\func_01&ven_10ec&dev_0256",
            "6.0.9600.1",
            "2026-04-10",
        ),
    ];

    let devices = inventory.inventory().unwrap();
    let found = FakeSource(updates).search().unwrap();
    let safe = filter_safe_updates(&devices, found);
    let deduped = dedup_updates(&devices, safe);

    assert!(
        !deduped.iter().any(|u| u.class == DeviceClass::Display),
        "no GPU/display updates should survive"
    );
    let audio: Vec<_> = deduped
        .iter()
        .filter(|u| u.class == DeviceClass::Audio)
        .collect();
    assert_eq!(audio.len(), 1, "duplicate Realtek update collapsed to one");

    let groups = group_by_class(deduped);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].class, DeviceClass::Audio);
}

#[test]
fn install_reports_progress_stages_and_success() {
    let src = FakeSource(vec![]);
    let mut stages = Vec::new();
    let report = src
        .install("Some-Update:1", &mut |p| stages.push(p.stage))
        .unwrap();
    assert_eq!(
        stages,
        vec![InstallStage::Downloading, InstallStage::Installing]
    );
    assert!(report.success);
    assert_eq!(report.result_code, 2);
}
