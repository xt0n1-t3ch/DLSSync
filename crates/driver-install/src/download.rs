use crate::DriverInstallError;
use dll_catalog::DownloadProgress;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_CHUNK_TIMEOUT: Duration = Duration::from_secs(60);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);
const RETRY_BACKOFF_MS: &[u64] = &[500, 2_000, 5_000];

#[derive(Clone)]
pub struct DownloadOpts {
    pub max_retries: u32,
    pub chunk_timeout: Duration,
    pub progress_tx: Option<mpsc::UnboundedSender<DownloadProgress>>,
    pub cancel: Option<CancellationToken>,
}

impl Default for DownloadOpts {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            chunk_timeout: DEFAULT_CHUNK_TIMEOUT,
            progress_tx: None,
            cancel: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub bytes: u64,
}

pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    opts: DownloadOpts,
) -> Result<DownloadOutcome, DriverInstallError> {
    let max_attempts = opts.max_retries.max(1);
    let mut last_err = String::new();
    for attempt in 1..=max_attempts {
        if is_cancelled(&opts) {
            return Err(DriverInstallError::Cancelled);
        }
        match download_once(client, url, dest, attempt, &opts).await {
            Ok(outcome) => return Ok(outcome),
            Err(DriverInstallError::Cancelled) => return Err(DriverInstallError::Cancelled),
            Err(err) => {
                let _ = tokio::fs::remove_file(dest).await;
                let retriable = is_retriable(&err) && attempt < max_attempts;
                tracing::warn!(attempt, url, retry = retriable, error = %err, "installer download attempt failed");
                if !retriable {
                    return Err(err);
                }
                last_err = err.to_string();
                let backoff = RETRY_BACKOFF_MS
                    .get((attempt - 1) as usize)
                    .copied()
                    .unwrap_or(5_000);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
        }
    }
    Err(DriverInstallError::Retries {
        attempts: max_attempts,
        last: last_err,
    })
}

async fn download_once(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    attempt: u32,
    opts: &DownloadOpts,
) -> Result<DownloadOutcome, DriverInstallError> {
    let response = client.get(url).send().await?.error_for_status()?;
    let expected = response.content_length();
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut written: u64 = 0;
    loop {
        if is_cancelled(opts) {
            return Err(DriverInstallError::Cancelled);
        }
        let next = tokio::time::timeout(opts.chunk_timeout, stream.next()).await;
        let chunk = match next {
            Ok(Some(chunk)) => chunk?,
            Ok(None) => break,
            Err(_) => {
                return Err(DriverInstallError::Stalled {
                    seconds: opts.chunk_timeout.as_secs(),
                })
            }
        };
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            emit(opts, url, written, expected, started, attempt);
            last_emit = Instant::now();
        }
    }
    file.flush().await?;
    emit(opts, url, written, expected, started, attempt);
    if let Some(expected) = expected {
        if written < expected {
            return Err(DriverInstallError::Truncated {
                got: written,
                expected,
            });
        }
    }
    Ok(DownloadOutcome {
        path: dest.to_path_buf(),
        bytes: written,
    })
}

fn emit(
    opts: &DownloadOpts,
    url: &str,
    written: u64,
    expected: Option<u64>,
    started: Instant,
    attempt: u32,
) {
    let Some(tx) = &opts.progress_tx else {
        return;
    };
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let _ = tx.send(DownloadProgress {
        url: url.to_string(),
        bytes_downloaded: written,
        bytes_total: expected,
        bytes_per_sec: (written as f64) / elapsed,
        attempt,
    });
}

fn is_cancelled(opts: &DownloadOpts) -> bool {
    opts.cancel.as_ref().is_some_and(|c| c.is_cancelled())
}

fn is_retriable(err: &DriverInstallError) -> bool {
    match err {
        DriverInstallError::Http(e) => {
            e.is_connect()
                || e.is_timeout()
                || e.is_body()
                || e.is_request()
                || matches!(
                    e.status().map(|s| s.as_u16()),
                    Some(408 | 429 | 500 | 502 | 503 | 504)
                )
        }
        DriverInstallError::Stalled { .. } | DriverInstallError::Truncated { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_token_short_circuits() {
        let token = CancellationToken::new();
        token.cancel();
        let opts = DownloadOpts {
            cancel: Some(token),
            ..Default::default()
        };
        assert!(is_cancelled(&opts));
    }

    #[test]
    fn default_opts_have_no_progress_or_cancel() {
        let opts = DownloadOpts::default();
        assert!(opts.progress_tx.is_none());
        assert!(opts.cancel.is_none());
        assert_eq!(opts.max_retries, DEFAULT_MAX_RETRIES);
    }
}
