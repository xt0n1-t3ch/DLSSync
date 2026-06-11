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

/// NVIDIA's GeForce/Display driver installer runs unattended with `/s` (silent)
/// and `/n` (no reboot) — no GUI, no setup wizard.
const NVIDIA_SILENT_ARGS: &[&str] = &["/s", "/n"];
/// Intel's Arc/graphics installer runs unattended with `/s`.
const INTEL_SILENT_ARGS: &[&str] = &["/s"];

/// Vendor-correct silent-install command line for a downloaded driver installer,
/// or an empty string when the vendor's installer cannot be driven silently (so
/// the caller launches its normal GUI). Matched case-insensitively on the vendor
/// name; an unknown vendor returns empty (GUI). The result is already safely
/// quoted via [`build_command_line`] and is suitable for
/// [`launch_and_wait_with_args`].
///
/// AMD is deliberately empty: the downloaded `amd-software-*.exe` is a
/// self-extractor that unpacks then chains to its own `Setup.exe -INSTALL`; the
/// outer stub does not honour a silent switch, so passing one is a no-op at best
/// and breaks extraction at worst. AMD therefore keeps its GUI until/unless a
/// verified two-stage silent path is implemented.
pub fn silent_install_args(vendor: &str) -> String {
    let parts: &[&str] = match vendor.trim().to_ascii_lowercase().as_str() {
        "nvidia" => NVIDIA_SILENT_ARGS,
        "intel" => INTEL_SILENT_ARGS,
        _ => &[],
    };
    build_command_line(parts)
}

/// Quote one argument per the Windows `CommandLineToArgvW` convention so it
/// survives as a single argv element regardless of spaces, tabs, or embedded
/// double-quotes. This is the canonical algorithm (backslashes are only special
/// immediately before a quote). Callers MUST build elevated command lines through
/// [`build_command_line`] rather than string-interpolating untrusted values —
/// raw interpolation is an argument-injection vector into the elevated child.
pub fn quote_arg(arg: &str) -> String {
    let needs_quotes = arg.is_empty()
        || arg
            .chars()
            .any(|c| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '"'));
    if !needs_quotes {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(ch);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

/// Join `args` into a single, safely-quoted `lpParameters` string for
/// [`launch_elevated`]. Every element is passed through [`quote_arg`].
pub fn build_command_line<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|a| quote_arg(a.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn launch_and_wait(
    installer: &Path,
    cancel: Option<&CancellationToken>,
) -> Result<i32, DriverInstallError> {
    launch_and_wait_with_args(installer, "", cancel)
}

/// Launch `installer` with `args` on its command line (the verb stays `open`, so
/// the installer raises its own UAC prompt if it needs elevation) and wait for it
/// to exit. `args` is a pre-built, safely-quoted parameter string — construct it
/// through [`build_command_line`], never by interpolating untrusted values. An
/// empty `args` runs the installer with no extra flags (identical to the old
/// behaviour). Used to pass vendor silent-install switches so a routine driver
/// update need not pop the full vendor GUI.
pub fn launch_and_wait_with_args(
    installer: &Path,
    args: &str,
    cancel: Option<&CancellationToken>,
) -> Result<i32, DriverInstallError> {
    let file: Vec<u16> = installer
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let params: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(params.as_ptr())
            },
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

#[cfg(test)]
mod tests {
    use super::{build_command_line, quote_arg, silent_install_args};

    #[test]
    fn nvidia_silent_args_are_slash_s_slash_n() {
        assert_eq!(silent_install_args("nvidia"), "/s /n");
        assert_eq!(silent_install_args("NVIDIA"), "/s /n");
        assert_eq!(silent_install_args("  Nvidia "), "/s /n");
    }

    #[test]
    fn intel_silent_args_are_slash_s() {
        assert_eq!(silent_install_args("intel"), "/s");
        assert_eq!(silent_install_args("Intel"), "/s");
    }

    #[test]
    fn amd_has_no_silent_args_self_extractor_keeps_gui() {
        assert_eq!(silent_install_args("amd"), "");
        assert_eq!(silent_install_args("AMD"), "");
    }

    #[test]
    fn unknown_vendor_falls_back_to_gui() {
        assert_eq!(silent_install_args("other"), "");
        assert_eq!(silent_install_args(""), "");
        assert_eq!(silent_install_args("microsoft"), "");
    }

    #[test]
    fn plain_args_are_unquoted() {
        assert_eq!(quote_arg("--wua-install"), "--wua-install");
        assert_eq!(quote_arg("{abc-123}:7"), "{abc-123}:7");
    }

    #[test]
    fn spaces_get_quoted() {
        assert_eq!(
            quote_arg("C:\\Program Files\\x.json"),
            "\"C:\\Program Files\\x.json\""
        );
    }

    #[test]
    fn embedded_quote_is_escaped_not_terminating() {
        // The classic injection payload tries to close the quoted token and inject
        // a flag. Every interior `"` must be emitted as an escaped `\"`, so
        // CommandLineToArgvW keeps the whole thing as one inert argv element.
        let payload = "abc\" --restore-driver \"C:\\evil";
        let quoted = quote_arg(payload);
        assert_eq!(quoted, "\"abc\\\" --restore-driver \\\"C:\\evil\"");
        assert!(quoted.starts_with('"') && quoted.ends_with('"'));
        // No bare (unescaped) interior quote exists: every `"` is preceded by `\`.
        let bytes = quoted.as_bytes();
        for i in 1..bytes.len() - 1 {
            if bytes[i] == b'"' {
                assert_eq!(bytes[i - 1], b'\\', "interior quote at {i} not escaped");
            }
        }
    }

    #[test]
    fn trailing_backslashes_are_doubled_before_close() {
        assert_eq!(quote_arg("a b\\\\"), "\"a b\\\\\\\\\"");
    }

    #[test]
    fn command_line_joins_each_arg_quoted() {
        let line = build_command_line(["--result", "C:\\Program Files\\r.json", "--id", "x\"y"]);
        assert_eq!(
            line,
            "--result \"C:\\Program Files\\r.json\" --id \"x\\\"y\""
        );
    }
}
