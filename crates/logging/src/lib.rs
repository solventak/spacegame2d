use std::{env, io, path::PathBuf};

use chrono::Utc;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Owns the background writer for one logging session.
pub struct SessionLogging {
    _file_guard: WorkerGuard,
}

/// Initialize human-readable or JSON logging to stdout and a per-run file.
pub fn init(binary_name: &str, default_filter: &str) -> io::Result<SessionLogging> {
    let directory = PathBuf::from(env::var("SPACEGAME_LOG_DIR").unwrap_or_else(|_| "logs".into()));
    std::fs::create_dir_all(&directory)?;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let filename = format!("{timestamp}_{binary_name}_{}.log", std::process::id());
    let file = tracing_appender::rolling::never(&directory, filename);
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_filter))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

    if env::var("SPACEGAME_LOG_FORMAT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json().with_target(false))
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(writer),
            )
            .try_init()
            .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error))?;
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().with_target(false))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(writer),
            )
            .try_init()
            .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error))?;
    }
    Ok(SessionLogging { _file_guard: guard })
}
