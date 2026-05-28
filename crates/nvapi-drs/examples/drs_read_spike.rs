fn main() {
    #[cfg(windows)]
    {
        match nvapi_drs::ffi::read_base_profile() {
            Ok(info) => {
                println!(
                    "NVAPI-DRS-SPIKE OK | base profile '{}' | apps={} settings={} | struct_ver=0x{:08X}",
                    info.name, info.num_apps, info.num_settings, info.struct_version
                );
            }
            Err(error) => {
                eprintln!("NVAPI-DRS-SPIKE FAILED | {error}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("NVAPI-DRS-SPIKE SKIPPED | windows-only");
        std::process::exit(2);
    }
}
