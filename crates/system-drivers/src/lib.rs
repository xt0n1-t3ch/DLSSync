//! General PC driver engine — audio, network, Bluetooth, input, storage,
//! chipset, USB, firmware, cameras, printers.
//!
//! Source of truth = the **Windows Update Agent (WUA) COM API** opted into the
//! **Microsoft Update Catalog**, inventoried per device through WMI
//! `Win32_PnPSignedDriver`. Only vendor-signed drivers Microsoft already
//! distributes are offered — no OEM-site scraping (that is what makes other
//! "driver booster" tools fragile and unsafe). An anti-downgrade guard refuses
//! any candidate that is not provably newer than the installed driver.
//!
//! The COM/WMI surface lives behind the [`DeviceCatalog`] and [`UpdateSource`]
//! traits so the orchestration (matching + anti-downgrade + grouping) is pure
//! and unit-tested without a live Windows Update service.

mod classify;
mod snapshot;
mod store;
mod version;

pub use classify::{classify, classify_best, DeviceClass};
pub use snapshot::{
    add_driver_install_args, export_driver_args, is_published_oem_inf, restore_inf_glob,
};
pub use store::{parse_enum_drivers, versions_by_original_name, DriverStorePackage};
pub use version::{extract_version, is_newer, ole_date_to_iso, DriverVersion};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(windows)]
mod inventory;
#[cfg(windows)]
mod wua;

#[cfg(windows)]
pub use inventory::WmiInventory;
#[cfg(windows)]
pub use snapshot::win as driver_snapshot;
#[cfg(windows)]
pub use wua::WuaSource;

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("device inventory failed: {0}")]
    Inventory(String),
    #[error("update search failed: {0}")]
    Search(String),
    #[error("install failed: {0}")]
    Install(String),
    #[error("update not found: {0}")]
    NotFound(String),
}

/// An installed device + its current driver, from WMI `Win32_PnPSignedDriver`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemDevice {
    pub name: String,
    pub class: DeviceClass,
    pub manufacturer: String,
    pub driver_version: Option<String>,
    /// ISO `YYYY-MM-DD`.
    pub driver_date: Option<String>,
    /// Raw hardware id (uppercased), e.g. `PCI\VEN_8086&DEV_9A49&SUBSYS_...`.
    pub hardware_id: String,
    /// The DriverStore published INF name for the installed driver, e.g.
    /// `oem47.inf`, from WMI `InfName`. The handle `pnputil /export-driver`
    /// needs to snapshot this device's current driver before an update.
    #[serde(default)]
    pub inf_name: Option<String>,
    /// Whether the device is currently present (vs a phantom/ghost of removed
    /// hardware). Defaults to `true` so older payloads and tests stay valid.
    #[serde(default = "default_present")]
    pub present: bool,
}

fn default_present() -> bool {
    true
}

/// A candidate driver offered by Windows Update / the Microsoft Update Catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverUpdate {
    /// `UpdateID:RevisionNumber` — the stable handle used to install.
    pub update_id: String,
    pub title: String,
    pub class: DeviceClass,
    pub provider: String,
    pub driver_version: Option<String>,
    /// ISO `YYYY-MM-DD` from `DriverVerDate`.
    pub driver_date: Option<String>,
    pub hardware_id: Option<String>,
    pub size_bytes: u64,
    /// Name of the installed device this update targets, when resolved.
    pub target_device: Option<String>,
    /// Installed driver version on the matched device, when resolved (for current→new display).
    pub current_version: Option<String>,
    /// DriverStore published INF (`oemNN.inf`) of the matched installed device,
    /// so the install can snapshot the current driver before replacing it.
    #[serde(default)]
    pub target_inf: Option<String>,
    /// Raw hardware id of the matched installed device (for the backup row).
    #[serde(default)]
    pub target_hardware_id: Option<String>,
    /// Vendor support / "more info" page exposed by Windows Update, when present.
    pub support_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStage {
    Downloading,
    Installing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub stage: InstallStage,
    pub message: String,
    pub fraction: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReport {
    pub success: bool,
    pub reboot_required: bool,
    pub result_code: i32,
    pub message: String,
}

/// Per-class group of available updates, for the UI's "System & Components" view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    pub class: DeviceClass,
    pub label: String,
    pub updates: Vec<DriverUpdate>,
}

/// Device inventory provider (WMI on Windows; fakeable in tests).
pub trait DeviceCatalog {
    fn inventory(&self) -> Result<Vec<SystemDevice>, DriverError>;
}

/// Driver update provider (WUA on Windows; fakeable in tests).
pub trait UpdateSource {
    fn search(&self) -> Result<Vec<DriverUpdate>, DriverError>;
    fn install(
        &self,
        update_id: &str,
        on_progress: &mut dyn FnMut(InstallProgress),
    ) -> Result<InstallReport, DriverError>;
}

/// Pick the first genuinely useful vendor/info URL from WUA's candidates
/// (`MoreInfoUrls` entries first, then `SupportUrl`). Windows Update very often
/// returns the generic, 404-ing support hub (`support.microsoft.com/.../select/
/// ?target=hub`); those — and non-http/empty values — are rejected so the UI can
/// fall back to a real Microsoft Update Catalog search instead.
pub fn pick_support_url(candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|s| s.trim())
        .find(|s| is_useful_support_url(s))
        .map(|s| s.to_string())
}

fn is_useful_support_url(u: &str) -> bool {
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return false;
    }
    !u.to_ascii_lowercase().contains("microsoft.com")
}

/// PCI vendor id of NVIDIA.
pub const VEN_NVIDIA: &str = "VEN_10DE";
/// PCI vendor id of AMD.
pub const VEN_AMD: &str = "VEN_1002";
/// PCI vendor id of Intel (covers Intel display-class iGPUs).
pub const VEN_INTEL: &str = "VEN_8086";

/// True when a hardware id + class describe a graphics adapter (discrete or
/// integrated) rather than a monitor panel. The dedicated GPU updater owns
/// these, so System & Components excludes them: a PCI `Display`-class device
/// whose vendor is one of the three GPU silicon vendors.
pub fn is_gpu(hardware_id: &str, class: DeviceClass) -> bool {
    if class != DeviceClass::Display {
        return false;
    }
    let s = hardware_id.to_ascii_uppercase();
    if !s.starts_with("PCI\\") {
        return false;
    }
    matches!(
        hwid_core(&s).map(|(v, _)| v),
        Some(v) if v == VEN_NVIDIA || v == VEN_AMD || v == VEN_INTEL
    )
}

/// Extract `(vendor_token, device_token)` — e.g. `("VEN_8086", "DEV_9A49")` or
/// `("VID_046D", "PID_C52B")` — from a hardware id, ignoring the `SUBSYS`/`REV`
/// /instance suffixes. Both tokens must come from the SAME `\`-delimited
/// segment so a compound id cannot pair a vendor from one device with a device
/// id from another. Returns `None` when no single segment carries both.
pub fn hwid_core(raw: &str) -> Option<(String, String)> {
    let s = raw.to_ascii_uppercase();
    for segment in s.split('\\') {
        let vendor = find_token(segment, &["VEN_", "VID_"]);
        let device = find_token(segment, &["DEV_", "PID_"]);
        if let (Some(v), Some(d)) = (vendor, device) {
            return Some((v, d));
        }
    }
    None
}

fn find_token(s: &str, prefixes: &[&str]) -> Option<String> {
    for p in prefixes {
        if let Some(idx) = s.find(p) {
            let start = idx;
            let rest = &s[idx + p.len()..];
            let hex: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            if !hex.is_empty() {
                return Some(format!("{}{}", &s[start..idx + p.len()], hex));
            }
        }
    }
    None
}

/// A normalized comparison key for an installed device or an update, broad
/// enough that the anti-downgrade guard fires on the device classes WUA most
/// often downgrades — chipset/ACPI, OEM-audio APO (`SWC\`), HD-audio codecs and
/// monitors — not just clean `PCI\VEN&DEV` hardware. Falls back to the leading
/// enumerator-specific segment so two nodes of the same physical device share a
/// key. `None` only for ids with no usable discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HwidKey {
    /// `(vendor_token, device_token)` from a single segment.
    VenDev(String, String),
    /// Monitor EDID id, e.g. `DISPLAY\DELA1A3` → `DELA1A3`.
    Monitor(String),
    /// ACPI / SWC / generic enumerator key, e.g. `ACPI\INTC1234` → `INTC1234`,
    /// `SWC\VEN_...&DEV_...` collapses via [`HwidKey::VenDev`].
    Enumerator(String, String),
}

/// Derive the broadened [`HwidKey`] used for matching + dedup.
pub fn hwid_key(raw: &str) -> Option<HwidKey> {
    if let Some((v, d)) = hwid_core(raw) {
        return Some(HwidKey::VenDev(v, d));
    }
    let s = raw.trim().to_ascii_uppercase();
    let mut parts = s.split('\\');
    let enumerator = parts.next()?;
    let descriptor = parts.next()?.trim();
    if descriptor.is_empty() {
        return None;
    }
    if enumerator == "DISPLAY" {
        return Some(HwidKey::Monitor(descriptor.to_string()));
    }
    Some(HwidKey::Enumerator(
        enumerator.to_string(),
        descriptor.to_string(),
    ))
}

/// True when an update's hardware id targets the given installed device, using
/// the broadened [`HwidKey`] so ACPI/SWC/monitor devices match (and are then
/// anti-downgrade checked) instead of silently passing unverified.
pub fn matches_device(update: &DriverUpdate, device: &SystemDevice) -> bool {
    match (
        update.hardware_id.as_deref().and_then(hwid_key),
        hwid_key(&device.hardware_id),
    ) {
        (Some(u), Some(d)) => u == d,
        _ => false,
    }
}

/// Drop phantom / non-present devices (ghosts of removed hardware) so updates
/// cannot match hardware that is no longer installed.
pub fn filter_present(devices: Vec<SystemDevice>) -> Vec<SystemDevice> {
    devices.into_iter().filter(|d| d.present).collect()
}

/// Stable dedup key for one physical device: its [`HwidKey`] when derivable,
/// else the leading two `\`-segments of the raw id (the enumerator + instance
/// prefix) so multi-node devices collapse to one.
fn device_dedup_key(hardware_id: &str) -> String {
    if let Some(key) = hwid_key(hardware_id) {
        return format!("{key:?}");
    }
    let s = hardware_id.to_ascii_uppercase();
    s.split('\\').take(2).collect::<Vec<_>>().join("\\")
}

/// Collapse multi-node inventory rows (HD-audio `FUNC_xx`, USB-composite
/// `MI_xx`, per-monitor INF entries) to one representative per physical device.
/// The representative keeps the richest `DeviceName` and the newest
/// `DriverDate`. Order of first appearance is preserved.
pub fn dedup_devices(devices: Vec<SystemDevice>) -> Vec<SystemDevice> {
    let mut order: Vec<String> = Vec::new();
    let mut best: std::collections::HashMap<String, SystemDevice> =
        std::collections::HashMap::new();
    for dev in devices {
        let key = device_dedup_key(&dev.hardware_id);
        match best.get_mut(&key) {
            None => {
                order.push(key.clone());
                best.insert(key, dev);
            }
            Some(cur) => {
                if dev.name.len() > cur.name.len() {
                    cur.name = dev.name.clone();
                }
                if dev.driver_date.as_deref() > cur.driver_date.as_deref() {
                    cur.driver_date = dev.driver_date.clone();
                    cur.driver_version = dev.driver_version.clone();
                    cur.inf_name = dev.inf_name.clone();
                }
            }
        }
    }
    order.into_iter().filter_map(|k| best.remove(&k)).collect()
}

/// Rank provider `a` against `b` for one device: `Less` when `a` is the better
/// match for the device manufacturer (so it sorts first), `Greater` when `b`
/// is, `Equal` when neither wins. An OEM/silicon-vendor provider outranks a
/// Microsoft-generic one via a normalized lowercase token-overlap.
fn provider_prefers(a: &str, b: &str, manufacturer: &str) -> std::cmp::Ordering {
    let score = |p: &str| brand_affinity(p, manufacturer);
    score(b).cmp(&score(a))
}

fn brand_affinity(provider: &str, manufacturer: &str) -> u8 {
    let p = provider.to_ascii_lowercase();
    let m = manufacturer.to_ascii_lowercase();
    if p.is_empty() {
        return 0;
    }
    if p.contains("microsoft") {
        return 1;
    }
    let token = m.split_whitespace().next().unwrap_or("");
    if !token.is_empty()
        && (p.contains(token) || m.contains(p.split_whitespace().next().unwrap_or("")))
    {
        return 3;
    }
    2
}

/// True when an update describes a GPU driver (so System & Components drops it;
/// the dedicated GPU updater owns it). An update is a GPU when its own hardware
/// id is a GPU, or when it resolves to a GPU among the installed devices.
fn update_is_gpu(update: &DriverUpdate, devices: &[SystemDevice]) -> bool {
    if let Some(hwid) = update.hardware_id.as_deref() {
        if is_gpu(hwid, update.class) {
            return true;
        }
    }
    devices
        .iter()
        .filter(|d| is_gpu(&d.hardware_id, d.class))
        .any(|d| matches_device(update, d))
}

/// Anti-downgrade guard: keep only updates that are provably newer than the
/// matched installed driver. GPU updates are excluded (the dedicated GPU
/// updater owns them). Updates with no resolvable installed device are kept
/// (WUA already deemed them applicable; we cannot prove a downgrade), and
/// annotated with the matched device name when there is one.
pub fn filter_safe_updates(
    devices: &[SystemDevice],
    updates: Vec<DriverUpdate>,
) -> Vec<DriverUpdate> {
    updates
        .into_iter()
        .filter_map(|mut up| {
            if update_is_gpu(&up, devices) {
                return None;
            }
            if let Some(dev) = devices.iter().find(|d| matches_device(&up, d)) {
                let newer = is_newer(
                    up.driver_date.as_deref(),
                    up.driver_version.as_deref(),
                    dev.driver_date.as_deref(),
                    dev.driver_version.as_deref(),
                );
                if !newer {
                    return None;
                }
                up.class = dev.class;
                up.target_device = Some(dev.name.clone());
                up.current_version = dev.driver_version.clone();
                up.target_inf = dev.inf_name.clone();
                up.target_hardware_id = Some(dev.hardware_id.clone());
            }
            Some(up)
        })
        .collect()
}

/// Collapse duplicate updates that target the same device. The Microsoft Update
/// Catalog routinely returns one driver several times (overlapping providers,
/// stale revisions); keep one winner per `(HwidKey-or-normalized-title)` group:
/// the OEM/silicon-vendor provider over a Microsoft-generic one (matched to the
/// installed device's manufacturer), then the newest by [`is_newer`]. Updates
/// with no derivable group key pass through untouched.
pub fn dedup_updates(devices: &[SystemDevice], updates: Vec<DriverUpdate>) -> Vec<DriverUpdate> {
    let manufacturer_of = |up: &DriverUpdate| -> String {
        devices
            .iter()
            .find(|d| matches_device(up, d))
            .map(|d| d.manufacturer.clone())
            .unwrap_or_default()
    };
    let group_key = |up: &DriverUpdate| -> Option<String> {
        if let Some(k) = up.hardware_id.as_deref().and_then(hwid_key) {
            return Some(format!("{k:?}"));
        }
        let t = up.title.to_ascii_lowercase();
        if t.trim().is_empty() {
            None
        } else {
            Some(t)
        }
    };

    let mut order: Vec<String> = Vec::new();
    let mut best: std::collections::HashMap<String, DriverUpdate> =
        std::collections::HashMap::new();
    let mut passthrough: Vec<DriverUpdate> = Vec::new();

    for up in updates {
        let Some(key) = group_key(&up) else {
            passthrough.push(up);
            continue;
        };
        match best.remove(&key) {
            None => {
                order.push(key.clone());
                best.insert(key, up);
            }
            Some(cur) => {
                let manufacturer = manufacturer_of(&up);
                let keep_new = match provider_prefers(&up.provider, &cur.provider, &manufacturer) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Greater => false,
                    std::cmp::Ordering::Equal => is_newer(
                        up.driver_date.as_deref(),
                        up.driver_version.as_deref(),
                        cur.driver_date.as_deref(),
                        cur.driver_version.as_deref(),
                    ),
                };
                best.insert(key, if keep_new { up } else { cur });
            }
        }
    }

    let mut out: Vec<DriverUpdate> = order.into_iter().filter_map(|k| best.remove(&k)).collect();
    out.append(&mut passthrough);
    out
}

/// Group safe updates by device class, newest-titled first within a group,
/// classes in their enum order. Empty classes are omitted.
pub fn group_by_class(updates: Vec<DriverUpdate>) -> Vec<DeviceGroup> {
    let mut by_class: BTreeMap<DeviceClass, Vec<DriverUpdate>> = BTreeMap::new();
    for up in updates {
        by_class.entry(up.class).or_default().push(up);
    }
    by_class
        .into_iter()
        .map(|(class, mut updates)| {
            updates.sort_by(|a, b| a.title.cmp(&b.title));
            DeviceGroup {
                class,
                label: class.label().to_string(),
                updates,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(name: &str, class: DeviceClass, hwid: &str, ver: &str, date: &str) -> SystemDevice {
        SystemDevice {
            name: name.into(),
            class,
            manufacturer: "Test".into(),
            driver_version: Some(ver.into()),
            driver_date: Some(date.into()),
            hardware_id: hwid.into(),
            inf_name: None,
            present: true,
        }
    }

    fn dev_mfg(
        name: &str,
        class: DeviceClass,
        hwid: &str,
        ver: &str,
        date: &str,
        manufacturer: &str,
    ) -> SystemDevice {
        SystemDevice {
            manufacturer: manufacturer.into(),
            ..dev(name, class, hwid, ver, date)
        }
    }

    fn upd(
        title: &str,
        class: DeviceClass,
        hwid: Option<&str>,
        ver: Option<&str>,
        date: &str,
    ) -> DriverUpdate {
        DriverUpdate {
            update_id: format!("{title}:1"),
            title: title.into(),
            class,
            provider: "Test".into(),
            driver_version: ver.map(Into::into),
            driver_date: Some(date.into()),
            hardware_id: hwid.map(Into::into),
            size_bytes: 1000,
            target_device: None,
            current_version: None,
            target_inf: None,
            target_hardware_id: None,
            support_url: None,
        }
    }

    #[test]
    fn pick_support_url_rejects_generic_hub_and_prefers_real() {
        assert_eq!(
            pick_support_url(&["https://support.microsoft.com/en-us/select/?target=hub".into()]),
            None
        );
        assert_eq!(
            pick_support_url(&[
                "https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-submission-support".into(),
                "http://sysdev.microsoft.com/support/default.aspx".into(),
            ]),
            None
        );
        assert_eq!(
            pick_support_url(&["https://www.dell.com/support/home/drivers/123".into()]).as_deref(),
            Some("https://www.dell.com/support/home/drivers/123")
        );
        assert_eq!(
            pick_support_url(&[
                "  ".into(),
                "https://support.microsoft.com/en-us/select/?target=hub".into(),
                "https://www.realtek.com/downloads/audio".into(),
            ])
            .as_deref(),
            Some("https://www.realtek.com/downloads/audio")
        );
        assert_eq!(
            pick_support_url(&["".into(), "ftp://x".into(), "javascript:1".into()]),
            None
        );
    }

    #[test]
    fn hwid_core_extracts_pair_ignoring_suffix() {
        assert_eq!(
            hwid_core(r"PCI\VEN_8086&DEV_9A49&SUBSYS_22128086&REV_01\3&11583659&0&10"),
            Some(("VEN_8086".into(), "DEV_9A49".into()))
        );
        assert_eq!(
            hwid_core(r"USB\VID_046D&PID_C52B\5&abc"),
            Some(("VID_046D".into(), "PID_C52B".into()))
        );
        assert_eq!(hwid_core("ROOT\\SYSTEM"), None);
    }

    #[test]
    fn matches_device_by_ven_dev_across_casing_and_suffix() {
        let d = dev(
            "Intel UHD",
            DeviceClass::Display,
            r"PCI\VEN_8086&DEV_9A49&SUBSYS_X&REV_01\inst",
            "31.0.101.999",
            "2026-04-06",
        );
        let u = upd(
            "Intel - Display - 31.0.101.2141",
            DeviceClass::Display,
            Some(r"pci\ven_8086&dev_9a49"),
            Some("31.0.101.2141"),
            "2026-05-15",
        );
        assert!(matches_device(&u, &d));
    }

    #[test]
    fn anti_downgrade_drops_older_and_equal() {
        let devices = vec![dev(
            "Intel Wi-Fi 6 AX201",
            DeviceClass::Network,
            r"PCI\VEN_8086&DEV_A0F0\x",
            "22.100.0.2141",
            "2026-04-06",
        )];
        let older = upd(
            "Intel - Net - 22.100.0.500",
            DeviceClass::Network,
            Some(r"pci\ven_8086&dev_a0f0"),
            Some("22.100.0.500"),
            "2026-01-01",
        );
        let same = upd(
            "Intel - Net - 22.100.0.2141",
            DeviceClass::Network,
            Some(r"pci\ven_8086&dev_a0f0"),
            Some("22.100.0.2141"),
            "2026-04-06",
        );
        let newer = upd(
            "Intel - Net - 22.100.0.8000",
            DeviceClass::Network,
            Some(r"pci\ven_8086&dev_a0f0"),
            Some("22.100.0.8000"),
            "2026-05-15",
        );

        let kept = filter_safe_updates(&devices, vec![older, same, newer]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].title, "Intel - Net - 22.100.0.8000");
        assert_eq!(
            kept[0].target_device.as_deref(),
            Some("Intel Wi-Fi 6 AX201")
        );
    }

    #[test]
    fn matched_device_class_overrides_vague_update_class() {
        let devices = vec![dev(
            "Realtek High Definition Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.1.1",
            "2025-01-01",
        )];
        let vague = upd(
            "A-Volute AudioProcessingObject Driver Update",
            DeviceClass::Other,
            Some(r"hdaudio\func_01&ven_10ec&dev_0256"),
            Some("1.1.4.0"),
            "2026-02-02",
        );
        let kept = filter_safe_updates(&devices, vec![vague]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].class, DeviceClass::Audio);
        assert_eq!(
            kept[0].target_device.as_deref(),
            Some("Realtek High Definition Audio")
        );
        assert_eq!(kept[0].current_version.as_deref(), Some("6.0.1.1"));
    }

    #[test]
    fn matched_update_carries_inf_and_hardware_id_for_snapshot() {
        let mut device = dev(
            "Realtek High Definition Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.1.1",
            "2025-01-01",
        );
        device.inf_name = Some("oem47.inf".into());
        let newer = upd(
            "Realtek - MEDIA - 6.0.2.0",
            DeviceClass::Audio,
            Some(r"hdaudio\func_01&ven_10ec&dev_0256"),
            Some("6.0.2.0"),
            "2026-04-01",
        );
        let kept = filter_safe_updates(&[device], vec![newer]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].target_inf.as_deref(), Some("oem47.inf"));
        assert_eq!(
            kept[0].target_hardware_id.as_deref(),
            Some(r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x")
        );
    }

    #[test]
    fn keeps_update_with_no_matching_device() {
        let devices = vec![dev(
            "Realtek Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.1.1",
            "2025-01-01",
        )];
        let net = upd(
            "Intel - Net - 12.19.2.50",
            DeviceClass::Network,
            Some(r"PCI\VEN_8086&DEV_15BB"),
            Some("12.19.2.50"),
            "2026-02-02",
        );
        let kept = filter_safe_updates(&devices, vec![net]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].target_device, None);
    }

    #[test]
    fn groups_by_class_sorted() {
        let updates = vec![
            upd("B audio", DeviceClass::Audio, None, None, "2026-01-01"),
            upd("A audio", DeviceClass::Audio, None, None, "2026-01-01"),
            upd("Net thing", DeviceClass::Network, None, None, "2026-01-01"),
        ];
        let groups = group_by_class(updates);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].class, DeviceClass::Audio);
        assert_eq!(groups[0].updates[0].title, "A audio");
        assert_eq!(groups[0].label, "Audio");
        assert_eq!(groups[1].class, DeviceClass::Network);
    }

    fn upd_provider(
        title: &str,
        class: DeviceClass,
        hwid: Option<&str>,
        ver: Option<&str>,
        date: &str,
        provider: &str,
    ) -> DriverUpdate {
        DriverUpdate {
            provider: provider.into(),
            ..upd(title, class, hwid, ver, date)
        }
    }

    #[test]
    fn is_gpu_only_for_pci_display_vendors() {
        assert!(is_gpu(
            r"PCI\VEN_10DE&DEV_2D05&SUBSYS_X\inst",
            DeviceClass::Display
        ));
        assert!(is_gpu(r"PCI\VEN_1002&DEV_164E", DeviceClass::Display));
        assert!(is_gpu(r"PCI\VEN_8086&DEV_9A49", DeviceClass::Display));
        assert!(!is_gpu(r"DISPLAY\DELA1A3\4&abc", DeviceClass::Monitor));
        assert!(!is_gpu(r"PCI\VEN_8086&DEV_A0F0", DeviceClass::Network));
        assert!(!is_gpu(r"PCI\VEN_1234&DEV_5678", DeviceClass::Display));
    }

    #[test]
    fn gpu_updates_excluded_monitor_kept() {
        let devices = vec![
            dev(
                "NVIDIA GeForce RTX",
                DeviceClass::Display,
                r"PCI\VEN_10DE&DEV_2D05\x",
                "560.0.0.0",
                "2026-01-01",
            ),
            dev(
                "Intel UHD",
                DeviceClass::Display,
                r"PCI\VEN_8086&DEV_9A49\x",
                "31.0.101.2141",
                "2026-01-01",
            ),
            dev(
                "Dell Monitor",
                DeviceClass::Monitor,
                r"DISPLAY\DELA1A3\4&edid",
                "1.0.0.0",
                "2024-01-01",
            ),
        ];
        let updates = vec![
            upd(
                "NVIDIA - Display - 565.0.0.0",
                DeviceClass::Display,
                Some(r"pci\ven_10de&dev_2d05"),
                Some("565.0.0.0"),
                "2026-05-01",
            ),
            upd(
                "Intel - Display - 31.0.101.8000",
                DeviceClass::Display,
                Some(r"pci\ven_8086&dev_9a49"),
                Some("31.0.101.8000"),
                "2026-05-01",
            ),
            upd(
                "Dell - Monitor - 1.1.0.0",
                DeviceClass::Monitor,
                Some(r"display\dela1a3"),
                Some("1.1.0.0"),
                "2026-05-01",
            ),
        ];
        let kept = filter_safe_updates(&devices, updates);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].class, DeviceClass::Monitor);
        assert_eq!(kept[0].target_device.as_deref(), Some("Dell Monitor"));
    }

    #[test]
    fn hwid_core_is_segment_aware() {
        assert_eq!(
            hwid_core(r"PCI\VEN_8086&DEV_9A49&SUBSYS_22128086&REV_01\3&11583659&0&10"),
            Some(("VEN_8086".into(), "DEV_9A49".into()))
        );
        let crafted = r"PCI\VEN_8086&SUBSYS_X\DEV_10DE_inst";
        assert_eq!(hwid_core(crafted), None);
    }

    #[test]
    fn hwid_key_broadens_acpi_swc_monitor() {
        assert_eq!(
            hwid_key(r"ACPI\INTC1234\3&inst"),
            Some(HwidKey::Enumerator("ACPI".into(), "INTC1234".into()))
        );
        assert_eq!(
            hwid_key(r"DISPLAY\DELA1A3\4&edid"),
            Some(HwidKey::Monitor("DELA1A3".into()))
        );
        assert_eq!(
            hwid_key(r"SWC\VEN_10EC&DEV_1234\x"),
            Some(HwidKey::VenDev("VEN_10EC".into(), "DEV_1234".into()))
        );
        assert_eq!(hwid_key("ROOT"), None);
    }

    #[test]
    fn anti_downgrade_fires_on_acpi_swc() {
        let devices = vec![dev(
            "Intel Chipset",
            DeviceClass::Chipset,
            r"ACPI\INTC1234\inst",
            "2.0.0.0",
            "2026-04-01",
        )];
        let older = upd(
            "Intel - System - 1.0.0.0",
            DeviceClass::System,
            Some(r"ACPI\INTC1234"),
            Some("1.0.0.0"),
            "2025-01-01",
        );
        let kept = filter_safe_updates(&devices, vec![older]);
        assert!(
            kept.is_empty(),
            "an older ACPI update must now be dropped by the anti-downgrade guard"
        );
    }

    #[test]
    fn dedup_devices_collapses_multifunction_and_composite() {
        let devices = vec![
            dev(
                "Realtek Audio",
                DeviceClass::Audio,
                r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\1",
                "6.0.1.1",
                "2025-01-01",
            ),
            dev(
                "Realtek Audio Codec",
                DeviceClass::Audio,
                r"HDAUDIO\FUNC_02&VEN_10EC&DEV_0256\2",
                "6.0.1.1",
                "2025-06-01",
            ),
            dev(
                "Logitech A",
                DeviceClass::Input,
                r"USB\VID_046D&PID_C52B&MI_00\3",
                "1.0",
                "2024-01-01",
            ),
            dev(
                "Logitech B",
                DeviceClass::Input,
                r"USB\VID_046D&PID_C52B&MI_01\4",
                "1.0",
                "2024-01-01",
            ),
        ];
        let out = dedup_devices(devices);
        assert_eq!(out.len(), 2);
        let audio = out.iter().find(|d| d.class == DeviceClass::Audio).unwrap();
        assert_eq!(audio.name, "Realtek Audio Codec");
        assert_eq!(audio.driver_date.as_deref(), Some("2025-06-01"));
    }

    #[test]
    fn dedup_updates_prefers_oem_then_newest() {
        let devices = vec![dev_mfg(
            "Realtek Audio",
            DeviceClass::Audio,
            r"HDAUDIO\FUNC_01&VEN_10EC&DEV_0256\x",
            "6.0.1.0",
            "2025-01-01",
            "Realtek Semiconductor Corp.",
        )];
        let generic = upd_provider(
            "Generic High Definition Audio",
            DeviceClass::Audio,
            Some(r"hdaudio\func_01&ven_10ec&dev_0256"),
            Some("10.0.0.1"),
            "2026-05-01",
            "Microsoft",
        );
        let oem = upd_provider(
            "Realtek - MEDIA - 6.0.2.0",
            DeviceClass::Audio,
            Some(r"hdaudio\func_01&ven_10ec&dev_0256"),
            Some("6.0.2.0"),
            "2026-04-01",
            "Realtek Semiconductor Corp.",
        );
        let out = dedup_updates(&devices, vec![generic, oem]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].provider, "Realtek Semiconductor Corp.");
    }

    #[test]
    fn dedup_updates_collapses_revisions_when_provider_equal() {
        let devices: Vec<SystemDevice> = vec![];
        let old_rev = upd(
            "Intel - Net - 22.100.0.1",
            DeviceClass::Network,
            Some(r"PCI\VEN_8086&DEV_A0F0"),
            Some("22.100.0.1"),
            "2026-01-01",
        );
        let new_rev = upd(
            "Intel - Net - 22.250.0.1",
            DeviceClass::Network,
            Some(r"PCI\VEN_8086&DEV_A0F0"),
            Some("22.250.0.1"),
            "2026-05-01",
        );
        let out = dedup_updates(&devices, vec![old_rev, new_rev]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].driver_version.as_deref(), Some("22.250.0.1"));
    }

    #[test]
    fn filter_present_drops_phantoms() {
        let mut ghost = dev(
            "Old Headset",
            DeviceClass::Audio,
            r"USB\VID_1234&PID_5678\x",
            "1.0",
            "2020-01-01",
        );
        ghost.present = false;
        let live = dev(
            "Current NIC",
            DeviceClass::Network,
            r"PCI\VEN_8086&DEV_A0F0\x",
            "1.0",
            "2026-01-01",
        );
        let out = filter_present(vec![ghost, live]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Current NIC");
    }

    #[test]
    fn cross_segment_pair_is_rejected() {
        let device = dev(
            "Some device",
            DeviceClass::Other,
            r"PCI\VEN_8086&SUBSYS_X\DEV_10DE_inst",
            "1.0",
            "2026-01-01",
        );
        let update = upd(
            "Mismatched",
            DeviceClass::Other,
            Some(r"PCI\VEN_8086&DEV_10DE"),
            Some("2.0"),
            "2026-05-01",
        );
        assert!(!matches_device(&update, &device));
    }
}
