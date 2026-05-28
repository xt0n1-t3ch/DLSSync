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

pub fn classify_exit(code: i32) -> InstallStage {
    match code {
        EXIT_OK | EXIT_REBOOT_REQUIRED => InstallStage::Completed,
        EXIT_USER_CANCELLED | EXIT_UAC_DECLINED => InstallStage::Cancelled,
        _ => InstallStage::Failed,
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
    fn exit_codes_map_to_terminal_stages() {
        assert_eq!(classify_exit(0), InstallStage::Completed);
        assert_eq!(classify_exit(3010), InstallStage::Completed);
        assert_eq!(classify_exit(1602), InstallStage::Cancelled);
        assert_eq!(classify_exit(1223), InstallStage::Cancelled);
        assert_eq!(classify_exit(1), InstallStage::Failed);
        assert_eq!(classify_exit(-1), InstallStage::Failed);
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
