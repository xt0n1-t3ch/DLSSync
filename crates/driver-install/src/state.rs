use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStage {
    Queued,
    Downloading,
    Verifying,
    Launching,
    Installing,
    Completed,
    Failed,
    Cancelled,
}

pub const INSTALL_STAGES: &[InstallStage] = &[
    InstallStage::Downloading,
    InstallStage::Verifying,
    InstallStage::Launching,
    InstallStage::Installing,
    InstallStage::Completed,
];

const EXIT_OK: i32 = 0;
const EXIT_REBOOT_REQUIRED: i32 = 3010;
const EXIT_USER_CANCELLED: i32 = 1602;
const EXIT_UAC_DECLINED: i32 = 1223;
/// Intel Graphics installer: "No driver was found that can be installed on the
/// current device" — the package's INF lists no id matching this GPU.
const EXIT_INTEL_NO_COMPATIBLE_DEVICE: i32 = 8;

impl InstallStage {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            InstallStage::Completed | InstallStage::Failed | InstallStage::Cancelled
        )
    }

    pub fn can_advance_to(self, next: InstallStage) -> bool {
        if self.is_terminal() {
            return false;
        }
        if matches!(next, InstallStage::Failed | InstallStage::Cancelled) {
            return true;
        }
        matches!(
            (self, next),
            (InstallStage::Queued, InstallStage::Downloading)
                | (InstallStage::Downloading, InstallStage::Verifying)
                | (InstallStage::Verifying, InstallStage::Launching)
                | (InstallStage::Launching, InstallStage::Installing)
                | (InstallStage::Installing, InstallStage::Completed)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallPhase {
    pub stage: InstallStage,
    pub message: String,
    pub progress: Option<f64>,
    pub error: Option<String>,
}

impl InstallPhase {
    pub fn new(stage: InstallStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
            progress: None,
            error: None,
        }
    }
}

pub fn reboot_required(code: i32) -> bool {
    code == EXIT_REBOOT_REQUIRED
}

pub fn classify_exit(code: i32) -> InstallStage {
    match code {
        EXIT_OK | EXIT_REBOOT_REQUIRED => InstallStage::Completed,
        EXIT_USER_CANCELLED | EXIT_UAC_DECLINED => InstallStage::Cancelled,
        _ => InstallStage::Failed,
    }
}

/// Human-facing message for a vendor installer exit code. Vendor is matched
/// case-insensitively ("intel"/"nvidia"/"amd"); unknown codes fall back to a
/// generic line that still surfaces the raw code for support.
pub fn describe_exit(code: i32, vendor: &str) -> String {
    match code {
        EXIT_OK => "Driver installed successfully.".to_string(),
        EXIT_REBOOT_REQUIRED => "Driver installed — restart your PC to finish.".to_string(),
        EXIT_USER_CANCELLED | EXIT_UAC_DECLINED => "Installation cancelled.".to_string(),
        EXIT_INTEL_NO_COMPATIBLE_DEVICE if vendor.eq_ignore_ascii_case("intel") => {
            "This Intel driver does not list your GPU (exit code 8). It is likely OEM-locked or needs a \
             different driver branch — get it from your laptop manufacturer or Windows Update."
                .to_string()
        }
        other => format!("Installer exited with code {other}."),
    }
}

pub fn parse_progress_percent(line: &str) -> Option<f64> {
    let bytes = line.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        if byte != b'%' {
            continue;
        }
        let mut start = index;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start == index {
            continue;
        }
        if let Ok(percent) = line[start..index].parse::<u32>() {
            if percent <= 100 {
                return Some(percent as f64 / 100.0);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions_are_allowed() {
        assert!(InstallStage::Queued.can_advance_to(InstallStage::Downloading));
        assert!(InstallStage::Downloading.can_advance_to(InstallStage::Verifying));
        assert!(InstallStage::Verifying.can_advance_to(InstallStage::Launching));
        assert!(InstallStage::Launching.can_advance_to(InstallStage::Installing));
        assert!(InstallStage::Installing.can_advance_to(InstallStage::Completed));
    }

    #[test]
    fn skipping_a_stage_is_rejected() {
        assert!(!InstallStage::Queued.can_advance_to(InstallStage::Verifying));
        assert!(!InstallStage::Downloading.can_advance_to(InstallStage::Installing));
    }

    #[test]
    fn failure_and_cancel_reachable_from_any_active_stage_only() {
        assert!(InstallStage::Downloading.can_advance_to(InstallStage::Failed));
        assert!(InstallStage::Installing.can_advance_to(InstallStage::Cancelled));
        assert!(!InstallStage::Completed.can_advance_to(InstallStage::Failed));
        assert!(!InstallStage::Cancelled.can_advance_to(InstallStage::Failed));
    }

    #[test]
    fn terminal_stages_do_not_advance() {
        for terminal in [
            InstallStage::Completed,
            InstallStage::Failed,
            InstallStage::Cancelled,
        ] {
            assert!(terminal.is_terminal());
            assert!(!terminal.can_advance_to(InstallStage::Installing));
        }
    }

    #[test]
    fn reboot_required_true_only_for_3010() {
        assert!(reboot_required(3010));
        assert!(!reboot_required(0));
        assert!(!reboot_required(1602));
        assert!(!reboot_required(1));
    }

    #[test]
    fn exit_codes_map_to_terminal_stages() {
        assert_eq!(classify_exit(0), InstallStage::Completed);
        assert_eq!(classify_exit(3010), InstallStage::Completed);
        assert_eq!(classify_exit(1602), InstallStage::Cancelled);
        assert_eq!(classify_exit(1223), InstallStage::Cancelled);
        assert_eq!(classify_exit(1), InstallStage::Failed);
        assert_eq!(classify_exit(-1), InstallStage::Failed);
    }

    #[test]
    fn describe_exit_explains_intel_code_8_only_for_intel() {
        let intel = describe_exit(8, "intel");
        assert!(intel.contains("exit code 8"));
        assert!(
            intel.to_lowercase().contains("oem-locked")
                || intel.to_lowercase().contains("windows update")
        );
        assert_eq!(describe_exit(8, "nvidia"), "Installer exited with code 8.");
    }

    #[test]
    fn describe_exit_covers_success_reboot_and_cancel() {
        assert_eq!(describe_exit(0, "nvidia"), "Driver installed successfully.");
        assert!(describe_exit(3010, "amd").contains("restart"));
        assert_eq!(describe_exit(1602, "intel"), "Installation cancelled.");
        assert_eq!(describe_exit(1223, "amd"), "Installation cancelled.");
        assert_eq!(describe_exit(5, "amd"), "Installer exited with code 5.");
    }

    #[test]
    fn parse_progress_reads_percent_tokens() {
        assert_eq!(
            parse_progress_percent("Installing display driver 47%"),
            Some(0.47)
        );
        assert_eq!(parse_progress_percent("100% complete"), Some(1.0));
        assert_eq!(parse_progress_percent("step 0% start"), Some(0.0));
        assert_eq!(parse_progress_percent("no percent here"), None);
        assert_eq!(parse_progress_percent("over 150% nonsense"), None);
        assert_eq!(parse_progress_percent("just a % sign"), None);
    }

    #[test]
    fn install_stages_describe_the_happy_path_in_order() {
        assert_eq!(INSTALL_STAGES.first(), Some(&InstallStage::Downloading));
        assert_eq!(INSTALL_STAGES.last(), Some(&InstallStage::Completed));
        assert_eq!(INSTALL_STAGES.len(), 5);
    }
}
