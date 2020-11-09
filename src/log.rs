use once_cell::sync::Lazy;
use slog::{o, Drain, FnValue, Logger, PushFnValue, Record};
use std::sync::Mutex;

pub static LOGGER: Lazy<slog::Logger> = Lazy::new(|| init_logger());

fn init_logger() -> Logger {
    let drain = slog_json::Json::new(std::io::stdout()).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(1000).build().fuse();

    slog::Logger::root(
        Mutex::new(drain).map(slog::Fuse),
        o!(
            "ts" => PushFnValue(move |_: &Record, ser| {
                ser.emit(chrono::Local::now().to_rfc3339())
            }),
            "lvl" => FnValue(move |rec: &Record| {
                rec.level().as_short_str()
            }),
            "loc" => FnValue(move |rec: &Record| {
                format!("{}:{}", rec.module(), rec.line())
            }),
            "msg" => PushFnValue(move |rec: &Record, ser| {
                ser.emit(rec.msg())
            }),
            "v" => env!("CARGO_PKG_VERSION"),
        ),
    )
}

#[macro_export]
macro_rules! debug(
    ($tag:expr, $($args:tt)*) => {
        slog::debug!($crate::log::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        slog::debug!($crate::log::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! info(
    ($tag:expr, $($args:tt)*) => {
        slog::info!($crate::log::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        slog::info!($crate::log::LOGGER, "{:?}", $($args)*)
    }
);

#[macro_export]
macro_rules! warn(
    ($tag:expr, $($args:tt)*) => {
        slog::warn!($crate::log::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        slog::warn!($crate::log::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! error(
    ($tag:expr, $($args:tt)*) => {
        slog::error!($crate::log::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        slog::error!($crate::log::LOGGER, "{:?}", $($args)*)
    };
);
