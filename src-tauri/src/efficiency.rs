#[cfg(windows)]
pub fn apply(enable: bool) -> Result<(), String> {
    win32_ecoqos::process::toggle_efficiency_mode(std::process::id(), Some(enable))
        .map_err(|e| format!("toggle EcoQoS: {e}"))
}

#[cfg(not(windows))]
pub fn apply(_enable: bool) -> Result<(), String> {
    Err("Efficiency Mode is Windows-only".to_string())
}
