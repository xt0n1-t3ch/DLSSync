pub mod download;
#[cfg(windows)]
pub mod launch;
pub mod state;
pub mod verify;

pub use download::{download_to_file, DownloadOpts, DownloadOutcome};
pub use state::{InstallPhase, InstallStage, INSTALL_STAGES};
pub use verify::verify_signature;

#[derive(Debug, thiserror::Error)]
pub enum DriverInstallError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("stalled: no bytes for {seconds} s")]
    Stalled { seconds: u64 },
    #[error("truncated: received {got} bytes of {expected}")]
    Truncated { got: u64, expected: u64 },
    #[error("oversized: stream exceeded the {cap}-byte cap")]
    Oversized { cap: u64 },
    #[error("deadline: download still running after {seconds} s")]
    Deadline { seconds: u64 },
    #[error("cancelled by user")]
    Cancelled,
    #[error("after {attempts} attempts: {last}")]
    Retries { attempts: u32, last: String },
    #[error("signature: {0}")]
    Signature(String),
    #[error("launch: {0}")]
    Launch(String),
}
