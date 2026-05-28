use crate::DriverInstallError;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use tokio_util::sync::CancellationToken;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const UAC_DECLINED_EXIT: i32 = 1223;
const WAIT_POLL_MS: u32 = 500;

pub fn launch_and_wait(
    installer: &Path,
    cancel: Option<&CancellationToken>,
) -> Result<i32, DriverInstallError> {
    let file: Vec<u16> = installer
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };

        if let Err(error) = ShellExecuteExW(&mut info) {
            if error.code() == ERROR_CANCELLED.to_hresult() {
                return Ok(UAC_DECLINED_EXIT);
            }
            return Err(DriverInstallError::Launch(format!(
                "ShellExecuteExW: {error}"
            )));
        }
        if info.hProcess.is_invalid() {
            return Err(DriverInstallError::Launch(
                "installer returned no process handle".to_string(),
            ));
        }

        loop {
            if cancel.is_some_and(|token| token.is_cancelled()) {
                let _ = CloseHandle(info.hProcess);
                return Err(DriverInstallError::Cancelled);
            }
            if WaitForSingleObject(info.hProcess, WAIT_POLL_MS) == WAIT_OBJECT_0 {
                break;
            }
        }

        let mut code: u32 = 0;
        let result = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        result
            .map_err(|error| DriverInstallError::Launch(format!("GetExitCodeProcess: {error}")))?;
        Ok(code as i32)
    }
}
