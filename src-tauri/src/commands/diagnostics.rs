use crate::constants::GITHUB_NEW_ISSUE_URL;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TAIL_LINES: usize = 400;
const MAX_TAIL_LINES: usize = 2000;
const ISSUE_TAIL_LINES: usize = 40;
const ISSUE_BODY_MAX_CHARS: usize = 5000;

#[derive(Debug, Clone, Serialize)]
pub struct LogPaths {
    pub logs_dir: String,
    pub current_log: Option<String>,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueReport {
    pub url: String,
    pub body: String,
}

fn resolve_logs_dir(state: &State<'_, AppState>) -> Option<PathBuf> {
    if let Some(paths) = state.paths.read().as_ref() {
        return Some(paths.logs_dir.clone());
    }
    crate::logging::logs_dir()
}

fn log_files(dir: &Path) -> Vec<(std::time::SystemTime, PathBuf)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("dlssync.log") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        out.push((modified, entry.path()));
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    out
}

fn tail(path: &Path, max: usize) -> std::io::Result<String> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max);
    Ok(lines[start..].join("\n"))
}

fn os_summary() -> String {
    let long = sysinfo::System::long_os_version();
    let build = sysinfo::System::kernel_version();
    match (long, build) {
        (Some(l), Some(b)) => format!("{l} (build {b})"),
        (Some(l), None) => l,
        _ => std::env::consts::OS.to_string(),
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn truncate_chars(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let kept: String = input.chars().take(max).collect();
    format!("{kept}\n…(truncated)")
}

#[tauri::command]
pub fn get_log_paths(state: State<'_, AppState>) -> AppResult<LogPaths> {
    let dir = resolve_logs_dir(&state)
        .ok_or_else(|| AppError::Other("logs directory unavailable".into()))?;
    let files = log_files(&dir);
    Ok(LogPaths {
        logs_dir: dir.to_string_lossy().into_owned(),
        current_log: files.first().map(|(_, p)| p.to_string_lossy().into_owned()),
        file_count: files.len(),
    })
}

#[tauri::command]
pub fn read_recent_logs(state: State<'_, AppState>, max_lines: Option<usize>) -> AppResult<String> {
    let dir = resolve_logs_dir(&state)
        .ok_or_else(|| AppError::Other("logs directory unavailable".into()))?;
    let Some((_, file)) = log_files(&dir).into_iter().next() else {
        return Ok(String::new());
    };
    let max = max_lines.unwrap_or(DEFAULT_TAIL_LINES).min(MAX_TAIL_LINES);
    tail(&file, max).map_err(|e| AppError::Other(format!("read log: {e}")))
}

#[tauri::command]
pub fn build_issue_report(
    state: State<'_, AppState>,
    context: Option<String>,
) -> AppResult<IssueReport> {
    let when = chrono::Utc::now().to_rfc3339();
    let log_tail = resolve_logs_dir(&state)
        .and_then(|dir| log_files(&dir).into_iter().next().map(|(_, p)| p))
        .and_then(|file| tail(&file, ISSUE_TAIL_LINES).ok())
        .unwrap_or_default();

    let context_block = match context.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => format!("\n### What happened\n\n{c}\n"),
        None => String::new(),
    };
    let log_block = if log_tail.trim().is_empty() {
        "_No recent log lines were captured._".to_string()
    } else {
        format!("<details><summary>Recent log (last {ISSUE_TAIL_LINES} lines)</summary>\n\n```\n{log_tail}\n```\n\n</details>")
    };

    let body = truncate_chars(
        &format!(
            "## Describe the problem\n\n_Replace this line with what went wrong and the steps to reproduce it._\n{context_block}\n---\n\n### Diagnostics\n\n- DLSSync version: `{APP_VERSION}`\n- OS: {os}\n- When: `{when}`\n\n{log_block}\n",
            os = os_summary(),
        ),
        ISSUE_BODY_MAX_CHARS,
    );

    let title = match context.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => format!("[bug] {}", c.lines().next().unwrap_or(c)),
        None => format!("[bug] Problem report from DLSSync v{APP_VERSION}"),
    };

    let url = format!(
        "{GITHUB_NEW_ISSUE_URL}?labels=bug&title={}&body={}",
        percent_encode(&truncate_chars(&title, 120)),
        percent_encode(&body),
    );

    Ok(IssueReport { url, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_keeps_unreserved_and_escapes_rest() {
        assert_eq!(percent_encode("abcXYZ09-_.~"), "abcXYZ09-_.~");
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("x&y=z"), "x%26y%3Dz");
        assert_eq!(percent_encode("line\n"), "line%0A");
    }

    #[test]
    fn percent_encode_handles_utf8_bytes() {
        assert_eq!(percent_encode("é"), "%C3%A9");
    }

    #[test]
    fn truncate_chars_caps_and_marks() {
        assert_eq!(truncate_chars("hello", 10), "hello");
        let out = truncate_chars("abcdef", 3);
        assert!(out.starts_with("abc"));
        assert!(out.contains("truncated"));
    }

    #[test]
    fn tail_returns_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("dlssync.log");
        std::fs::write(&f, "l1\nl2\nl3\nl4\nl5\n").unwrap();
        assert_eq!(tail(&f, 2).unwrap(), "l4\nl5");
        assert_eq!(tail(&f, 99).unwrap(), "l1\nl2\nl3\nl4\nl5");
    }

    #[test]
    fn log_files_sorts_newest_first_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("dlssync.log.2026-05-24"), b"a").unwrap();
        std::fs::write(dir.path().join("dlssync.log.2026-05-25"), b"b").unwrap();
        std::fs::write(dir.path().join("unrelated.txt"), b"c").unwrap();
        let files = log_files(dir.path());
        assert_eq!(files.len(), 2);
    }
}
