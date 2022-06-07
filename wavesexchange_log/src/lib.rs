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

/// Use this macro to set up a scope timer,
/// which logs execution time of a code block.
///
/// ```no_run
/// # use wavesexchange_log::timer;
/// {
///     timer!("this is a test code block");
///     // Some computations goes here
/// } // At the end of the scope the execution time is logged.
/// ```
///
/// When not specified, logging level `debug` is used by default,
/// as in the example above.
///
/// Logging level can be set explicitly, either `trace`, `debug` or `info`:
/// ```no_run
/// # use wavesexchange_log::timer;
/// timer!("this is a test", level = info);
/// timer!("this is a test", level = debug);
/// timer!("this is a test", level = trace);
/// ```
///
/// Also, verbose mode can be specified.
/// In this mode there will be two log records:
/// one when operation starts, and the second when it finishes.
///
/// ```no_run
/// # use wavesexchange_log::timer;
/// {
///     timer!("this is a test code block", verbose);
///     // Some computations goes here
/// } // At the end of the scope the execution time is logged.
/// ```
///
/// If logging level is not specified, `trace` will be used by default
/// for verbose mode.
///
/// If both logging level and verbose mode are set,
/// the logging level must come first:
///
/// ```no_run
/// # use wavesexchange_log::timer;
/// timer!("this is a test", level = info, verbose);
/// timer!("this is a test", level = debug, verbose);
/// timer!("this is a test", level = trace, verbose);
/// ```
#[macro_export]
macro_rules! timer {
    ($name:literal) => {
        $crate::timer!($name, level = debug)
    };
    ($name:literal, verbose) => {
        $crate::timer!($name, level = trace, verbose)
    };
    ($name:literal, level = trace) => {
        $crate::timer!(@ $name, $crate::slog::Level::Trace, false)
    };
    ($name:literal, level = debug) => {
        $crate::timer!(@ $name, $crate::slog::Level::Debug, false)
    };
    ($name:literal, level = info) => {
        $crate::timer!(@ $name, $crate::slog::Level::Info, false)
    };
    ($name:literal, level = trace, verbose) => {
        $crate::timer!(@ $name, $crate::slog::Level::Trace, true)
    };
    ($name:literal, level = debug, verbose) => {
        $crate::timer!(@ $name, $crate::slog::Level::Debug, true)
    };
    ($name:literal, level = info, verbose) => {
        $crate::timer!(@ $name, $crate::slog::Level::Info, true)
    };
    (@ $name:literal, $level:expr, $verbose:literal) => {
        let _timer = $crate::scopetimer::ScopeTimer::new($name, $level, $verbose);
    };
}

pub mod scopetimer {
    use slog::Level;
    use std::{fmt, time};

    pub struct ScopeTimer(&'static str, Level, bool, time::Instant);

    impl ScopeTimer {
        #[inline(always)]
        pub fn new(name: &'static str, level: Level, verbose: bool) -> Self {
            if verbose {
                print(level, format_args!("BEGIN {}", name));
            }
            ScopeTimer(name, level, verbose, time::Instant::now())
        }
    }

    impl Drop for ScopeTimer {
        #[inline(always)]
        fn drop(&mut self) {
            let &mut ScopeTimer(name, level, verbose, ref started) = self;
            let elapsed = started.elapsed();
            const MS_IN_SEC: f64 = 1_000.0;
            let elapsed_ms = elapsed.as_secs_f64() * MS_IN_SEC;
            if verbose {
                print(
                    level,
                    format_args!("END   {}: elapsed {}ms", name, elapsed_ms),
                );
            } else {
                print(
                    level,
                    format_args!("{}: completed in {}ms", name, elapsed_ms),
                );
            }
        }
    }

    #[inline(always)]
    fn print(level: Level, msg: fmt::Arguments) {
        match level {
            Level::Trace => super::trace!("{}", msg),
            Level::Debug => super::debug!("{}", msg),
            Level::Info => super::info!("{}", msg),
            _ => panic!("Bad log level for scope timer"),
        }
    }
}

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
