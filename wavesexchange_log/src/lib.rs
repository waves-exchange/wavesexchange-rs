pub use ::slog;

use crate::format::OutputFormat;
use once_cell::sync::Lazy;
use slog::{o, Drain, FnValue, Logger, PushFnValue, Record};
use std::sync::Mutex;

pub static LOGGER: Lazy<slog::Logger> = Lazy::new(|| init_logger());

fn init_logger() -> Logger {
    match OutputFormat::from_env() {
        OutputFormat::PlainText => {
            let decorator = slog_term::PlainDecorator::new(std::io::stdout());
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_async::Async::new(drain).chan_size(1000).build().fuse();
            let drain = slog_envlogger::new(drain).fuse();
            let drain = Mutex::new(drain).map(slog::Fuse);
            slog::Logger::root(drain, o!())
        }
        OutputFormat::Json => {
            let drain = slog_json::Json::new(std::io::stdout()).build().fuse();
            let drain = slog_async::Async::new(drain).chan_size(1000).build().fuse();
            let drain = slog_envlogger::new(drain).fuse();
            let drain = Mutex::new(drain).map(slog::Fuse);
            slog::Logger::root(
                drain,
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
    }
}

#[macro_export]
macro_rules! trace(
    ($arg:literal) => {
        $crate::slog::trace!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::trace!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::trace!($crate::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! debug(
    ($arg:literal) => {
        $crate::slog::debug!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::debug!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::debug!($crate::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! info(
    ($arg:literal) => {
        $crate::slog::info!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::info!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::info!($crate::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! warn(
    ($arg:literal) => {
        $crate::slog::warn!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::warn!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::warn!($crate::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! error(
    ($arg:literal) => {
        $crate::slog::error!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::error!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::error!($crate::LOGGER, "{:?}", $($args)*)
    };
);

#[macro_export]
macro_rules! crit(
    ($arg:literal) => {
        $crate::slog::crit!($crate::LOGGER, "{}", $arg)
    };
    ($tag:expr, $($args:tt)*) => {
        $crate::slog::crit!($crate::LOGGER, $tag, $($args)*)
    };
    ($($args:tt)*) => {
        $crate::slog::crit!($crate::LOGGER, "{:?}", $($args)*)
    };
);

mod format {
    use std::env;

    #[derive(Copy, Clone)]
    pub(crate) enum OutputFormat {
        PlainText,
        Json,
    }

    impl Default for OutputFormat {
        fn default() -> Self {
            Self::Json
        }
    }

    impl<S: AsRef<str>> From<S> for OutputFormat {
        fn from(s: S) -> Self {
            match s.as_ref() {
                "plain" => Self::PlainText,
                "json" => Self::Json,
                "" => Default::default(),
                _ => panic!("Unrecognized {} value: '{}'", Self::ENV_NAME, s.as_ref()),
            }
        }
    }

    impl OutputFormat {
        const ENV_NAME: &'static str = "RUST_LOG_FORMAT";

        pub(crate) fn from_env() -> Self {
            Self::from(env::var(Self::ENV_NAME).ok().unwrap_or_default())
        }
    }
}
