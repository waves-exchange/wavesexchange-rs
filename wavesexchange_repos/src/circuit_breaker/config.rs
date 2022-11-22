use std::time::Duration;

use serde::Deserialize;

fn default_max_timespan_ms() -> u64 {
    10000
}

fn default_max_err_count_per_timespan() -> usize {
    5
}

#[derive(Deserialize)]
struct ConfigFlat {
    #[serde(default = "default_max_timespan_ms")]
    max_timespan_ms: u64,
    #[serde(default = "default_max_err_count_per_timespan")]
    max_err_count_per_timespan: usize,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub max_timespan: Duration,
    pub max_err_count_per_timespan: usize,
}

pub fn load() -> Result<Config, envy::Error> {
    let config_flat = envy::prefixed("CIRCUIT_BREAKER_").from_env::<ConfigFlat>()?;

    Ok(Config {
        max_timespan: Duration::from_millis(config_flat.max_timespan_ms),
        max_err_count_per_timespan: config_flat.max_err_count_per_timespan,
    })
}
