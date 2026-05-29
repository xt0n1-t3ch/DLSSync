use crate::DriverInstallError;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub const UAC_DECLINED_EXIT: i32 = 1223;
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

/// Launch a fresh ELEVATED instance of `exe` (UAC "runas" verb) with `args` on
/// its command line, wait for it to exit, and return the exit code. Returns
/// `UAC_DECLINED_EXIT` (1223) when the user dismisses the UAC prompt. Used to run
/// Administrator-only work (e.g. WUA driver install) out-of-process while the
/// main app stays unelevated.
///
/// `on_tick` fires once per poll (~`WAIT_POLL_MS`) so the caller can stream
/// progress (e.g. read the child's progress file) while the child runs. `cancel`
/// and `timeout` bound the wait: on either, the elevated child is
/// `TerminateProcess`d (never orphaned) and the call returns
/// `DriverInstallError::Cancelled` or a timeout `Launch` error — so the UI can
/// never freeze forever on a stuck install or UAC dialog.
pub fn launch_elevated(
    exe: &Path,
    args: &str,
    cancel: Option<&CancellationToken>,
    timeout: Option<Duration>,
    mut on_tick: impl FnMut(),
) -> Result<i32, DriverInstallError> {
    let file: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
    let params: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };

        if let Err(error) = ShellExecuteExW(&mut info) {
            if error.code() == ERROR_CANCELLED.to_hresult() {
                return Ok(UAC_DECLINED_EXIT);
            }
            return Err(DriverInstallError::Launch(format!(
                "ShellExecuteExW(runas): {error}"
            )));
        }
        if info.hProcess.is_invalid() {
            return Err(DriverInstallError::Launch(
                "elevated helper returned no process handle".to_string(),
            ));
        }

        let start = Instant::now();
        loop {
            if cancel.is_some_and(|token| token.is_cancelled()) {
                let _ = TerminateProcess(info.hProcess, UAC_DECLINED_EXIT as u32);
                let _ = CloseHandle(info.hProcess);
                return Err(DriverInstallError::Cancelled);
            }
            if let Some(limit) = timeout {
                if start.elapsed() >= limit {
                    let _ = TerminateProcess(info.hProcess, 1);
                    let _ = CloseHandle(info.hProcess);
                    return Err(DriverInstallError::Launch(format!(
                        "elevated installer timed out after {}s",
                        limit.as_secs()
                    )));
                }
            }
            on_tick();
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
