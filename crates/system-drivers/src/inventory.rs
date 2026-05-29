//! Windows device inventory via WMI `Win32_PnPSignedDriver`.

use std::collections::HashMap;

use crate::{classify, dedup_devices, filter_present, DeviceCatalog, DriverError, SystemDevice};

type WmiRow = HashMap<String, wmi::Variant>;

/// Live WMI-backed device catalog.
pub struct WmiInventory;

impl DeviceCatalog for WmiInventory {
    fn inventory(&self) -> Result<Vec<SystemDevice>, DriverError> {
        let rows = query(
            "SELECT DeviceName, DeviceClass, Manufacturer, DriverVersion, DriverDate, DeviceID, \
             InfName FROM Win32_PnPSignedDriver",
        )
        .map_err(DriverError::Inventory)?;
        let devices: Vec<SystemDevice> = rows.iter().filter_map(device_from_row).collect();
        Ok(dedup_devices(filter_present(devices)))
    }
}

fn query(q: &str) -> Result<Vec<WmiRow>, String> {
    let com = wmi::COMLibrary::new().map_err(|e| format!("COM: {e}"))?;
    let conn = wmi::WMIConnection::new(com).map_err(|e| format!("WMI: {e}"))?;
    conn.raw_query(q).map_err(|e| format!("query: {e}"))
}

/// Map a single WMI row into a [`SystemDevice`]. Rows lacking both a name and a
/// hardware id are skipped (virtual/placeholder entries).
fn device_from_row(row: &WmiRow) -> Option<SystemDevice> {
    let hardware_id = string_field(row, "DeviceID")?.to_ascii_uppercase();
    let name = string_field(row, "DeviceName")
        .or_else(|| string_field(row, "Manufacturer"))
        .unwrap_or_else(|| "Unknown device".to_string());
    let class = classify(&string_field(row, "DeviceClass").unwrap_or_default());
    let manufacturer = string_field(row, "Manufacturer").unwrap_or_default();
    let driver_version = string_field(row, "DriverVersion").filter(|s| !s.is_empty());
    let driver_date = string_field(row, "DriverDate").and_then(|s| cim_to_iso(&s));
    let inf_name = string_field(row, "InfName").map(|s| s.to_ascii_lowercase());

    Some(SystemDevice {
        name,
        class,
        manufacturer,
        driver_version,
        driver_date,
        hardware_id,
        inf_name,
        present: true,
    })
}

fn string_field(row: &WmiRow, key: &str) -> Option<String> {
    match row.get(key)? {
        wmi::Variant::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Convert a WMI CIM_DATETIME (`yyyymmddHHMMSS.ffffff±UUU`) into ISO
/// `YYYY-MM-DD`. Returns `None` for malformed/empty input.
pub fn cim_to_iso(cim: &str) -> Option<String> {
    let digits: String = cim.chars().take(8).collect();
    if digits.len() != 8 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (y, rest) = digits.split_at(4);
    let (m, d) = rest.split_at(2);
    if m == "00" || d == "00" {
        return None;
    }
    Some(format!("{y}-{m}-{d}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cim_datetime() {
        assert_eq!(
            cim_to_iso("20260406000000.000000-000").as_deref(),
            Some("2026-04-06")
        );
        assert_eq!(
            cim_to_iso("20251231235959.999999+000").as_deref(),
            Some("2025-12-31")
        );
        assert_eq!(cim_to_iso("").as_deref(), None);
        assert_eq!(cim_to_iso("garbage").as_deref(), None);
        assert_eq!(cim_to_iso("20260000000000.000000-000").as_deref(), None);
    }

    #[test]
    fn maps_row_to_device() {
        let mut row: WmiRow = HashMap::new();
        row.insert(
            "DeviceID".into(),
            wmi::Variant::String(r"pci\ven_8086&dev_9a49\x".into()),
        );
        row.insert(
            "DeviceName".into(),
            wmi::Variant::String("Intel(R) UHD Graphics".into()),
        );
        row.insert("DeviceClass".into(), wmi::Variant::String("DISPLAY".into()));
        row.insert(
            "Manufacturer".into(),
            wmi::Variant::String("Intel Corporation".into()),
        );
        row.insert(
            "DriverVersion".into(),
            wmi::Variant::String("31.0.101.2141".into()),
        );
        row.insert(
            "DriverDate".into(),
            wmi::Variant::String("20260406000000.000000-000".into()),
        );

        let d = device_from_row(&row).expect("device");
        assert_eq!(d.hardware_id, r"PCI\VEN_8086&DEV_9A49\X");
        assert_eq!(d.class, classify::DeviceClass::Display);
        assert_eq!(d.driver_version.as_deref(), Some("31.0.101.2141"));
        assert_eq!(d.driver_date.as_deref(), Some("2026-04-06"));
    }

    #[test]
    fn skips_row_without_hardware_id() {
        let mut row: WmiRow = HashMap::new();
        row.insert("DeviceName".into(), wmi::Variant::String("Phantom".into()));
        assert!(device_from_row(&row).is_none());
    }
}
