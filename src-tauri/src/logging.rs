use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

const DEFAULT_FILTER: &str =
    "dlssync=info,launcher_scan=info,dll_catalog=info,backup_store=info,notifications_store=info";

pub fn logs_dir() -> Option<PathBuf> {
    crate::paths::default_root().map(|root| root.join("Logs"))
}

pub fn init() -> Option<WorkerGuard> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());
    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .compact();

    let dir = logs_dir().filter(|d| std::fs::create_dir_all(d).is_ok());
    match dir {
        Some(dir) => {
            let appender = tracing_appender::rolling::daily(&dir, "dlssync.log");
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let file_layer = fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_writer(writer);
            tracing_subscriber::registry()
                .with(filter())
                .with(stdout_layer)
                .with(file_layer)
                .try_init()
                .ok();
            Some(guard)
        }
        None => {
            tracing_subscriber::registry()
                .with(filter())
                .with(stdout_layer)
                .try_init()
                .ok();
            None
        }
    }
}
