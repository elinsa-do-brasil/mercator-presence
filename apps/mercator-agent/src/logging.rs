use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::config;

pub fn init() -> Result<WorkerGuard> {
    let log_dir = config::ensure_log_dir()?;
    let file_appender = tracing_appender::rolling::never(log_dir, "agent.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("mercator_agent=info"));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(guard)
}
