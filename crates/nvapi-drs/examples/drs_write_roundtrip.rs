fn main() {
    #[cfg(windows)]
    {
        use nvapi_drs::settings::ids;
        let value = 0x0000_000B;
        match nvapi_drs::ffi::roundtrip_dword(ids::DLSS_SR_FORCED_PRESET, value) {
            Ok(read_back) => {
                if read_back == value {
                    println!(
                        "NVAPI-WRITE-ROUNDTRIP OK | set+read 0x{value:08X} on DLSS_SR_FORCED_PRESET (session-only, not saved)"
                    );
                } else {
                    eprintln!(
                        "NVAPI-WRITE-ROUNDTRIP MISMATCH | set 0x{value:08X} read 0x{read_back:08X}"
                    );
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("NVAPI-WRITE-ROUNDTRIP FAILED | {error}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("NVAPI-WRITE-ROUNDTRIP SKIPPED | windows-only");
        std::process::exit(2);
    }
}
