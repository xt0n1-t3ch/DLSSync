pub mod pci {
    pub const NVIDIA: u16 = 0x10DE;
    pub const INTEL: u16 = 0x8086;
    pub const AMD: u16 = 0x1002;
}

pub mod nvidia {
    pub const AJAX_DRIVER_LOOKUP: &str =
        "https://gfwsl.geforce.com/services_toolkit/services/com/nvidia/services/AjaxDriverService.php";
    pub const GPU_DATA_URL: &str =
        "https://raw.githubusercontent.com/ZenitH-AT/nvidia-data/main/gpu-data.json";
    pub const OS_DATA_URL: &str =
        "https://raw.githubusercontent.com/ZenitH-AT/nvidia-data/main/os-data.json";
    pub const PUBLISHER_SUBJECT: &str = "NVIDIA Corporation";
    pub const LANGUAGE_CODE_EN_US: u32 = 1033;
    pub const OS_ID_WIN10_X64: u32 = 57;
    pub const OS_ID_WIN11_X64: u32 = 135;
}

pub mod intel {
    pub const DSA_CATALOG_URL: &str = "https://dsadata.intel.com/data/en";
    pub const PUBLISHER_SUBJECT: &str = "Intel Corporation";
    pub const SOFTWARE_CONFIGURATIONS_FILE: &str = "software-configurations.json";
    pub const SELECTABLE_GRAPHICS_FILE: &str = "selectable-graphics.json";
    pub const CLIENT_GRAPHICS_GROUP_ID: &str = "785597";
    pub const GRAPHICS_VERSION_PREFIX: &str = "32.0.101.";
}

pub mod amd {
    pub const VERSION_TABLE_URL: &str = "https://gpuopen.com/version-table/";
    pub const VERSION_TABLE_XML: &str =
        "https://raw.githubusercontent.com/GPUOpen-Drivers/amd-vulkan-versions/master/amdversions.xml";
    pub const DRIVER_DOWNLOAD_PREFIX: &str = "https://drivers.amd.com/drivers/";
    pub const PUBLISHER_SUBJECT: &str = "Advanced Micro Devices, Inc.";
}
