use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::filter::targets::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

pub enum LogLevel {
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        use LogLevel::*;
        match level {
            Off => LevelFilter::OFF,
            Trace => LevelFilter::TRACE,
            Debug => LevelFilter::DEBUG,
            Info => LevelFilter::INFO,
            Warn => LevelFilter::WARN,
            Error => LevelFilter::ERROR,
        }
    }
}

pub fn init_logger(level: LogLevel) {
    Registry::default()
        .with(tracing_subscriber::fmt::layer().with_filter(Targets::new().with_default(level)))
        .try_init()
        .ok();
    info!(
        "Welcome to ledger v{} «Dr Faustus»",
        env!("CARGO_PKG_VERSION")
    );
}
