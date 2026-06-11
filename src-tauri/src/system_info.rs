use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub ram: RamInfo,
    pub gpus: Vec<GpuInfo>,
    pub collected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub build: String,
    pub edition: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CpuInfo {
    pub brand: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RamModule {
    pub capacity_bytes: u64,
    pub mhz: u32,
    pub type_label: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RamInfo {
    pub total_bytes: u64,
    pub modules: Vec<RamModule>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub pci_vendor_id: u16,
    pub pci_device_id: u16,
    pub model: String,
    pub driver_version: String,
    pub vram_bytes: u64,
    pub recommended_runtimes: Vec<String>,
    /// NVIDIA driver model: DCH (Universal, the only model NVIDIA ships for RTX
    /// 20/30/40/50) vs the legacy Standard package. Drives the correct download
    /// URL flavour. Defaults to `true` (DCH) and only flips to `false` when
    /// `pnputil` resolves a known Standard INF, so a detection miss never sends a
    /// modern card to the wrong (Standard) installer.
    pub is_dch: bool,
    /// `false` when DXGI reported a zero `DeviceId`, so the adapter cannot be
    /// keyed to a `Win32_VideoController` row by PCI hardware-id. Driver-version
    /// enrichment is then skipped rather than guessed by a fuzzy name match that
    /// can land on the wrong row on a hybrid (iGPU + dGPU) laptop.
    pub identifiable: bool,
    /// RDNA4 (Radeon RX 9000) — the only generation AMD ships the FSR 4
    /// upscaler model for today. Gates 4.x FSR set offers and applies.
    pub fsr4_capable: bool,
}

/// Navi 4x parts allocate PCI device ids in 0x7550-0x75FF (Navi 48 = 0x7550
/// RX 9070/9070 XT, Navi 44 = 0x7590 RX 9060 line); the model-name check
/// covers adapters whose DXGI device id is missing or zero.
pub fn supports_fsr4(vendor: GpuVendor, device_id: u16, model: &str) -> bool {
    if !matches!(vendor, GpuVendor::Amd) {
        return false;
    }
    if matches!(device_id, 0x7550..=0x75FF) {
        return true;
    }
    model.to_ascii_lowercase().contains("radeon rx 9")
}

/// Windows PCI hardware-id fragment as it appears in a `PNPDeviceID` or an Intel
/// `DetectionValues` entry (e.g. `VEN_10DE&DEV_2705`). Uppercase, 4-hex padded.
pub fn hardware_id(vendor_id: u16, device_id: u16) -> String {
    format!("VEN_{vendor_id:04X}&DEV_{device_id:04X}")
}

pub fn collect() -> SystemInfo {
    SystemInfo {
        os: collect_os(),
        cpu: collect_cpu(),
        ram: collect_ram(),
        gpus: collect_gpus(),
        collected_at: chrono::Utc::now(),
    }
}

fn collect_os() -> OsInfo {
    let mut info = OsInfo {
        name: sysinfo::System::name().unwrap_or_else(|| "Unknown".into()),
        version: sysinfo::System::os_version().unwrap_or_else(|| "Unknown".into()),
        build: sysinfo::System::kernel_version().unwrap_or_else(|| "Unknown".into()),
        edition: String::new(),
    };
    #[cfg(windows)]
    {
        if let Ok(rows) =
            wmi_query("SELECT Caption, Version, BuildNumber FROM Win32_OperatingSystem")
        {
            if let Some(row) = rows.first() {
                if let Some(caption) = row.get("Caption").and_then(variant_as_string) {
                    info.edition = caption.trim().to_string();
                }
                if let Some(v) = row.get("Version").and_then(variant_as_string) {
                    info.version = v;
                }
                if let Some(b) = row.get("BuildNumber").and_then(variant_as_string) {
                    info.build = b;
                }
            }
        }
    }
    info
}

fn collect_cpu() -> CpuInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    let logical = sys.cpus().len();
    let brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown".into());
    let physical = sys.physical_core_count().unwrap_or(logical);
    CpuInfo {
        brand,
        physical_cores: physical,
        logical_cores: logical,
    }
}

fn collect_ram() -> RamInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory();
    let modules = collect_ram_modules();
    RamInfo {
        total_bytes: total,
        modules,
    }
}

#[cfg(windows)]
fn collect_ram_modules() -> Vec<RamModule> {
    let mut out = Vec::new();
    if let Ok(rows) = wmi_query(
        "SELECT Capacity, ConfiguredClockSpeed, Speed, SMBIOSMemoryType, MemoryType \
         FROM Win32_PhysicalMemory",
    ) {
        for row in rows {
            let capacity = row
                .get("Capacity")
                .and_then(variant_as_u64)
                .unwrap_or_default();
            let configured = row
                .get("ConfiguredClockSpeed")
                .and_then(variant_as_u64)
                .map(|v| v as u32)
                .unwrap_or(0);
            let speed = row
                .get("Speed")
                .and_then(variant_as_u64)
                .map(|v| v as u32)
                .unwrap_or(0);
            let mhz = if configured > 0 { configured } else { speed };
            let smbios = row
                .get("SMBIOSMemoryType")
                .and_then(variant_as_u64)
                .unwrap_or(0);
            let legacy = row.get("MemoryType").and_then(variant_as_u64).unwrap_or(0);
            let type_label = ddr_label(smbios as u32, legacy as u32);
            out.push(RamModule {
                capacity_bytes: capacity,
                mhz,
                type_label,
            });
        }
    }
    out
}

#[cfg(not(windows))]
fn collect_ram_modules() -> Vec<RamModule> {
    Vec::new()
}

fn ddr_label(smbios: u32, legacy: u32) -> String {
    let resolved = if smbios > 0 { smbios } else { legacy };
    match resolved {
        20 => "DDR".into(),
        21 => "DDR2".into(),
        24 => "DDR3".into(),
        26 => "DDR4".into(),
        28 => "LPDDR".into(),
        29 => "LPDDR2".into(),
        30 => "LPDDR3".into(),
        31 => "LPDDR4".into(),
        34 => "DDR5".into(),
        35 => "LPDDR5".into(),
        _ => "Unknown".into(),
    }
}

fn collect_gpus() -> Vec<GpuInfo> {
    let mut out = collect_gpus_dxgi();
    dedupe_adapters(&mut out);
    enrich_drivers(&mut out);
    out
}

/// Drop adapters that resolve to the same physical GPU. DXGI can surface a card
/// more than once; identical twin cards also collapse to one driver entry. Keyed
/// by `(vendor, device, model)` so two distinct GPUs are never merged.
fn dedupe_adapters(gpus: &mut Vec<GpuInfo>) {
    let mut seen = std::collections::BTreeSet::new();
    gpus.retain(|g| seen.insert((g.pci_vendor_id, g.pci_device_id, g.model.clone())));
}

#[cfg(windows)]
fn collect_gpus_dxgi() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_DESC1,
        DXGI_ADAPTER_FLAG_SOFTWARE,
    };
    let mut out = Vec::new();
    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(_) => return out,
        };
        let mut index = 0u32;
        loop {
            let adapter: IDXGIAdapter1 = match factory.EnumAdapters1(index) {
                Ok(a) => a,
                Err(_) => break,
            };
            let desc: DXGI_ADAPTER_DESC1 = match adapter.GetDesc1() {
                Ok(d) => d,
                Err(_) => {
                    index += 1;
                    continue;
                }
            };
            let is_software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
            if is_software {
                index += 1;
                continue;
            }
            let model = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .trim()
                .to_string();
            let vendor = vendor_from_id(desc.VendorId);
            let device_id = desc.DeviceId as u16;
            let fsr4_capable = supports_fsr4(vendor, device_id, &model);
            out.push(GpuInfo {
                vendor,
                pci_vendor_id: desc.VendorId as u16,
                pci_device_id: device_id,
                model,
                driver_version: "Unknown".into(),
                vram_bytes: desc.DedicatedVideoMemory as u64,
                recommended_runtimes: recommended_for(vendor),
                is_dch: true,
                identifiable: device_id != 0,
                fsr4_capable,
            });
            index += 1;
        }
    }
    out
}

#[cfg(not(windows))]
fn collect_gpus_dxgi() -> Vec<GpuInfo> {
    Vec::new()
}

fn vendor_from_id(vendor_id: u32) -> GpuVendor {
    match vendor_id {
        0x10DE => GpuVendor::Nvidia,
        0x1002 | 0x1022 => GpuVendor::Amd,
        0x8086 => GpuVendor::Intel,
        _ => GpuVendor::Other,
    }
}

const RUNTIME_DLSS_SR: &str = "DLSS Super Resolution";
const RUNTIME_DLSS_FG: &str = "DLSS Frame Generation";
const RUNTIME_DLSS_RR: &str = "DLSS Ray Reconstruction";
const RUNTIME_NVIDIA_REFLEX: &str = "NVIDIA Reflex";
const RUNTIME_NVIDIA_STREAMLINE: &str = "NVIDIA Streamline";
const RUNTIME_FSR_UPSCALING: &str = "FSR Upscaling";
const RUNTIME_FSR_FG: &str = "FSR Frame Generation";
const RUNTIME_INTEL_XESS: &str = "Intel XeSS";
const RUNTIME_XESS_FG: &str = "XeSS Frame Generation";
const RUNTIME_DIRECTSTORAGE: &str = "DirectStorage";

pub fn recommended_for(vendor: GpuVendor) -> Vec<String> {
    match vendor {
        GpuVendor::Nvidia => vec![
            RUNTIME_DLSS_SR.into(),
            RUNTIME_DLSS_FG.into(),
            RUNTIME_DLSS_RR.into(),
            RUNTIME_NVIDIA_REFLEX.into(),
        ],
        GpuVendor::Amd => vec![RUNTIME_FSR_UPSCALING.into(), RUNTIME_FSR_FG.into()],
        GpuVendor::Intel => vec![RUNTIME_INTEL_XESS.into(), RUNTIME_XESS_FG.into()],
        // An unrecognised GPU still runs the vendor-neutral families: XeSS (works
        // on any DX12 GPU), DirectStorage, and the cross-vendor NVIDIA Reflex /
        // Streamline runtimes — so unsupported-GPU users get usable suggestions
        // instead of XeSS alone.
        GpuVendor::Other => vec![
            RUNTIME_INTEL_XESS.into(),
            RUNTIME_DIRECTSTORAGE.into(),
            RUNTIME_NVIDIA_REFLEX.into(),
            RUNTIME_NVIDIA_STREAMLINE.into(),
        ],
    }
}

#[cfg(windows)]
type WmiRow = std::collections::HashMap<String, wmi::Variant>;

#[cfg(windows)]
fn enrich_drivers(gpus: &mut [GpuInfo]) {
    let rows = match wmi_query(
        "SELECT Name, DriverVersion, PNPDeviceID, InfFilename FROM Win32_VideoController",
    ) {
        Ok(r) => r,
        Err(_) => return,
    };
    let store = enum_driver_packages();
    for gpu in gpus.iter_mut() {
        // B10: an unidentifiable adapter (DXGI gave DeviceId 0) cannot be keyed by
        // PCI hardware-id; never enrich it from a fuzzy name guess that can match
        // the wrong row on a hybrid iGPU + dGPU laptop.
        if !gpu.identifiable {
            continue;
        }
        let Some(row) = row_for_gpu(gpu, &rows) else {
            continue;
        };
        if let Some(version) = row.get("DriverVersion").and_then(variant_as_string) {
            gpu.driver_version = version;
        }
        if gpu.vendor == GpuVendor::Nvidia {
            let inf = row.get("InfFilename").and_then(variant_as_string);
            gpu.is_dch = nvidia_is_dch(inf.as_deref(), &store);
        }
    }
}

/// Enumerate the local DriverStore via `pnputil /enum-drivers` (works
/// unelevated, reusing `system_drivers::parse_enum_drivers`). Used to resolve a
/// `Win32_VideoController.InfFilename` `oemNN.inf` back to its original vendor
/// INF name. Returns an empty list when `pnputil` is unavailable, in which case
/// DCH classification falls back to its default.
#[cfg(windows)]
fn enum_driver_packages() -> Vec<system_drivers::DriverStorePackage> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match std::process::Command::new("pnputil.exe")
        .args(["/enum-drivers"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(out) => system_drivers::parse_enum_drivers(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Resolve the original (vendor) INF name for a `Win32_VideoController`
/// `InfFilename`. When the value is already an original name it is returned as-is;
/// when it is a store `oemNN.inf` the matching DriverStore package's
/// `original_name` is returned. `None` when it cannot be resolved.
#[cfg(windows)]
fn resolve_original_inf(inf: &str, store: &[system_drivers::DriverStorePackage]) -> Option<String> {
    if !system_drivers::is_published_oem_inf(inf) {
        return Some(inf.to_ascii_lowercase());
    }
    store
        .iter()
        .find(|p| p.published_name.eq_ignore_ascii_case(inf))
        .map(|p| p.original_name.to_ascii_lowercase())
        .filter(|s| !s.is_empty())
}

/// Classify an NVIDIA GPU's driver model from its (resolved) original INF name.
/// DCH (Universal) packages publish `nv_dispi.inf` / `nvmd*.inf`; the legacy
/// Standard packages publish `nv_disp.inf` / `nvlt*.inf` / `nvaci.inf`. Defaults
/// to DCH (`true`) — the only model NVIDIA ships for RTX 20/30/40/50 — and only
/// returns `false` when a known Standard INF resolves, so a detection miss never
/// downgrades a modern card to the Standard installer.
fn nvidia_is_dch(inf: Option<&str>, store: &[StorePackage]) -> bool {
    let Some(raw) = inf.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let original = resolve_inf_for_classify(raw, store).unwrap_or_else(|| raw.to_ascii_lowercase());
    classify_nvidia_inf(&original)
}

/// Pure name → DCH classifier. Standard INFs win the comparison so an ambiguous
/// name defaults to DCH.
fn classify_nvidia_inf(original: &str) -> bool {
    const STANDARD_PREFIXES: [&str; 3] = ["nvlt", "nv_disp", "nvaci"];
    const DCH_PREFIXES: [&str; 2] = ["nv_dispi", "nvmd"];
    let name = original.trim().to_ascii_lowercase();
    // `nv_dispi` (DCH) starts with `nv_disp` (Standard), so test the longer/more
    // specific DCH prefixes first and only then the Standard ones.
    if DCH_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return true;
    }
    if STANDARD_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    true
}

#[cfg(windows)]
type StorePackage = system_drivers::DriverStorePackage;

#[cfg(windows)]
fn resolve_inf_for_classify(inf: &str, store: &[StorePackage]) -> Option<String> {
    resolve_original_inf(inf, store)
}

#[cfg(not(windows))]
type StorePackage = ();

#[cfg(not(windows))]
fn resolve_inf_for_classify(_inf: &str, _store: &[StorePackage]) -> Option<String> {
    None
}

/// Pick the `Win32_VideoController` row for this adapter. Prefers an exact PCI
/// hardware-id match against `PNPDeviceID` — the only correct key when two GPUs
/// of the same vendor (or an iGPU + dGPU) are present — and falls back to fuzzy
/// model-name containment only when no row carries a matching id. The caller has
/// already guaranteed `pci_device_id != 0` (see [`enrich_drivers`]).
#[cfg(windows)]
fn row_for_gpu<'a>(gpu: &GpuInfo, rows: &'a [WmiRow]) -> Option<&'a WmiRow> {
    let want = hardware_id(gpu.pci_vendor_id, gpu.pci_device_id);
    let by_id = rows.iter().find(|row| {
        row.get("PNPDeviceID")
            .and_then(variant_as_string)
            .map(|pnp| pnp.to_ascii_uppercase().contains(&want))
            .unwrap_or(false)
    });
    if by_id.is_some() {
        return by_id;
    }
    let needle = gpu.model.to_ascii_lowercase();
    rows.iter().find(|row| {
        let name = row
            .get("Name")
            .and_then(variant_as_string)
            .unwrap_or_default()
            .to_ascii_lowercase();
        !name.is_empty() && (name.contains(&needle) || needle.contains(&name))
    })
}

#[cfg(not(windows))]
fn enrich_drivers(_gpus: &mut [GpuInfo]) {}

#[cfg(windows)]
fn wmi_query(query: &str) -> Result<Vec<std::collections::HashMap<String, wmi::Variant>>, String> {
    let com = wmi::COMLibrary::new().map_err(|e| format!("COM: {e}"))?;
    let conn = wmi::WMIConnection::new(com).map_err(|e| format!("WMI: {e}"))?;
    conn.raw_query(query).map_err(|e| format!("query: {e}"))
}

#[cfg(windows)]
fn variant_as_string(v: &wmi::Variant) -> Option<String> {
    match v {
        wmi::Variant::String(s) => Some(s.clone()),
        wmi::Variant::I4(n) => Some(n.to_string()),
        wmi::Variant::UI4(n) => Some(n.to_string()),
        wmi::Variant::I8(n) => Some(n.to_string()),
        wmi::Variant::UI8(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(windows)]
fn variant_as_u64(v: &wmi::Variant) -> Option<u64> {
    match v {
        wmi::Variant::UI8(n) => Some(*n),
        wmi::Variant::UI4(n) => Some(*n as u64),
        wmi::Variant::I8(n) => Some(*n as u64),
        wmi::Variant::I4(n) => Some(*n as u64),
        wmi::Variant::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddr_label_smbios_codes() {
        assert_eq!(ddr_label(26, 0), "DDR4");
        assert_eq!(ddr_label(34, 0), "DDR5");
        assert_eq!(ddr_label(24, 0), "DDR3");
    }

    #[test]
    fn ddr_label_falls_back_to_legacy_when_smbios_zero() {
        assert_eq!(ddr_label(0, 26), "DDR4");
        assert_eq!(ddr_label(0, 24), "DDR3");
    }

    #[test]
    fn ddr_label_unknown_for_unmapped_code() {
        assert_eq!(ddr_label(99, 0), "Unknown");
        assert_eq!(ddr_label(0, 0), "Unknown");
    }

    fn gpu(vendor: GpuVendor, vid: u16, did: u16, model: &str) -> GpuInfo {
        GpuInfo {
            vendor,
            pci_vendor_id: vid,
            pci_device_id: did,
            model: model.into(),
            driver_version: "Unknown".into(),
            vram_bytes: 0,
            recommended_runtimes: vec![],
            is_dch: true,
            identifiable: did != 0,
            fsr4_capable: supports_fsr4(vendor, did, model),
        }
    }

    #[test]
    fn fsr4_capability_is_rdna4_only() {
        assert!(supports_fsr4(
            GpuVendor::Amd,
            0x7550,
            "AMD Radeon RX 9070 XT"
        ));
        assert!(supports_fsr4(GpuVendor::Amd, 0x7590, "AMD Radeon RX 9060"));
        assert!(supports_fsr4(GpuVendor::Amd, 0, "AMD Radeon RX 9070"));
        assert!(!supports_fsr4(
            GpuVendor::Amd,
            0x744C,
            "AMD Radeon RX 7900 XTX"
        ));
        assert!(!supports_fsr4(
            GpuVendor::Amd,
            0x73BF,
            "AMD Radeon RX 6800 XT"
        ));
        assert!(!supports_fsr4(
            GpuVendor::Nvidia,
            0x7550,
            "NVIDIA GeForce RTX 4070 Ti SUPER"
        ));
    }

    #[test]
    fn dedupe_adapters_collapses_identical_gpus_but_keeps_distinct_ones() {
        let mut gpus = vec![
            gpu(
                GpuVendor::Nvidia,
                0x10DE,
                0x2705,
                "NVIDIA GeForce RTX 4070 Ti SUPER",
            ),
            gpu(
                GpuVendor::Nvidia,
                0x10DE,
                0x2705,
                "NVIDIA GeForce RTX 4070 Ti SUPER",
            ),
            gpu(GpuVendor::Intel, 0x8086, 0x9A49, "Intel Iris Xe Graphics"),
        ];
        dedupe_adapters(&mut gpus);
        assert_eq!(gpus.len(), 2);
        assert!(gpus.iter().any(|g| g.vendor == GpuVendor::Intel));
        assert_eq!(gpus.iter().filter(|g| g.pci_device_id == 0x2705).count(), 1);
    }

    #[test]
    fn hardware_id_is_uppercase_4_hex_padded() {
        assert_eq!(hardware_id(0x10DE, 0x2705), "VEN_10DE&DEV_2705");
        assert_eq!(hardware_id(0x8086, 0x9A49), "VEN_8086&DEV_9A49");
        assert_eq!(hardware_id(0x8086, 0x46A6), "VEN_8086&DEV_46A6");
        assert_eq!(hardware_id(0x1002, 0x0B), "VEN_1002&DEV_000B");
    }

    #[test]
    fn vendor_from_id_maps_known_vendors() {
        assert_eq!(vendor_from_id(0x10DE), GpuVendor::Nvidia);
        assert_eq!(vendor_from_id(0x1002), GpuVendor::Amd);
        assert_eq!(vendor_from_id(0x8086), GpuVendor::Intel);
    }

    #[test]
    fn vendor_from_id_returns_other_for_unknown() {
        assert_eq!(vendor_from_id(0xFFFF), GpuVendor::Other);
        assert_eq!(vendor_from_id(0x0000), GpuVendor::Other);
    }

    #[test]
    fn recommended_for_nvidia_includes_dlss() {
        let recs = recommended_for(GpuVendor::Nvidia);
        assert!(recs.iter().any(|r| r.contains("DLSS")));
        assert!(recs.iter().any(|r| r.contains("Reflex")));
    }

    #[test]
    fn recommended_for_amd_includes_fsr() {
        let recs = recommended_for(GpuVendor::Amd);
        assert!(recs.iter().any(|r| r.contains("FSR")));
        assert!(!recs.iter().any(|r| r.contains("DLSS")));
    }

    #[test]
    fn recommended_for_intel_includes_xess() {
        let recs = recommended_for(GpuVendor::Intel);
        assert!(recs.iter().any(|r| r.contains("XeSS")));
    }

    #[test]
    fn recommended_for_other_includes_vendor_neutral_families() {
        let recs = recommended_for(GpuVendor::Other);
        assert!(recs.iter().any(|r| r == "Intel XeSS"));
        assert!(recs.iter().any(|r| r == "DirectStorage"));
        assert!(recs.iter().any(|r| r == "NVIDIA Reflex"));
        assert!(recs.iter().any(|r| r == "NVIDIA Streamline"));
        // An unsupported GPU must not be told it can run vendor-exclusive upscalers.
        assert!(!recs.iter().any(|r| r.contains("DLSS")));
        assert!(!recs.iter().any(|r| r.contains("FSR")));
    }

    #[test]
    fn classify_nvidia_inf_recognizes_dch_packages() {
        assert!(classify_nvidia_inf("nv_dispi.inf"));
        assert!(classify_nvidia_inf("nvmdi.inf"));
        assert!(classify_nvidia_inf("NVMDIG.INF"));
    }

    #[test]
    fn classify_nvidia_inf_recognizes_standard_packages() {
        assert!(!classify_nvidia_inf("nvlt.inf"));
        assert!(!classify_nvidia_inf("nvltwa.inf"));
        assert!(!classify_nvidia_inf("nv_disp.inf"));
        assert!(!classify_nvidia_inf("nvaci.inf"));
    }

    #[test]
    fn classify_nvidia_inf_prefers_dch_for_the_nv_dispi_vs_nv_disp_overlap() {
        // `nv_dispi` (DCH) has `nv_disp` (Standard) as a prefix — the more specific
        // DCH match must win so a DCH card is never misread as Standard.
        assert!(classify_nvidia_inf("nv_dispi.inf"));
        assert!(!classify_nvidia_inf("nv_disp.inf"));
    }

    #[test]
    fn classify_nvidia_inf_defaults_to_dch_for_unknown_names() {
        assert!(classify_nvidia_inf("oem42.inf"));
        assert!(classify_nvidia_inf(""));
        assert!(classify_nvidia_inf("something_else.inf"));
    }

    #[test]
    fn nvidia_is_dch_defaults_to_true_when_inf_is_absent() {
        let store: Vec<StorePackage> = Vec::new();
        assert!(nvidia_is_dch(None, &store));
        assert!(nvidia_is_dch(Some(""), &store));
        assert!(nvidia_is_dch(Some("   "), &store));
    }

    #[test]
    fn nvidia_is_dch_flips_to_false_for_a_standard_inf() {
        let store: Vec<StorePackage> = Vec::new();
        assert!(!nvidia_is_dch(Some("nvlt.inf"), &store));
        assert!(nvidia_is_dch(Some("nv_dispi.inf"), &store));
    }

    #[test]
    fn unidentifiable_gpu_has_identifiable_false() {
        let g = gpu(GpuVendor::Nvidia, 0x10DE, 0, "NVIDIA GeForce RTX 4090");
        assert!(!g.identifiable);
        let ok = gpu(GpuVendor::Nvidia, 0x10DE, 0x2684, "NVIDIA GeForce RTX 4090");
        assert!(ok.identifiable);
    }
}
