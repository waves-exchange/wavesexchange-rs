pub mod config;

pub use config::Config;
use wavesexchange_log::debug;

use std::{
    future::Future,
    mem::drop,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub trait FallibleDataSource {
    type Error;

    fn is_countable_err(err: &Self::Error) -> bool;

    fn fallback(&self, elapsed_ms: u128, err_count: usize) -> Self::Error {
        panic!(
            "CircuitBreaker panicked after {err_count} errors in a row happened in {elapsed_ms}ms"
        )
    }
}

pub trait DataSrcInitFn<S: FallibleDataSource>:
    Fn() -> Result<S, S::Error> + Send + Sync + 'static
{
}

impl<T, S: FallibleDataSource> DataSrcInitFn<S> for T where
    T: Fn() -> Result<S, S::Error> + Send + Sync + 'static
{
}

pub struct CircuitBreaker<S: FallibleDataSource> {
    max_timespan: Duration,
    max_err_count_per_timespan: usize,
    init_fn: Box<dyn DataSrcInitFn<S>>,
    state: RwLock<CBState<S>>,
}

pub struct CBState<S: FallibleDataSource> {
    data_source: Arc<S>,
    err_count: usize,
    first_err_ts: Option<Instant>,
}

impl<S: FallibleDataSource> CBState<S> {
    fn inc(&mut self) {
        self.err_count += 1;
    }

    fn reset(&mut self) {
        self.err_count = 0;
        self.first_err_ts = None;
    }

    fn reinit(&mut self, src: S) {
        self.data_source = Arc::new(src);
    }
}

pub struct CircuitBreakerBuilder<S: FallibleDataSource> {
    max_timespan: Option<Duration>,
    max_err_count_per_timespan: Option<usize>,
    init_fn: Option<Box<dyn DataSrcInitFn<S>>>,
}

impl<S: FallibleDataSource> CircuitBreakerBuilder<S> {
    pub fn new() -> CircuitBreakerBuilder<S> {
        CircuitBreakerBuilder {
            max_timespan: None,
            max_err_count_per_timespan: None,
            init_fn: None,
        }
    }

    pub fn with_max_timespan(mut self, ts: Duration) -> CircuitBreakerBuilder<S> {
        self.max_timespan = Some(ts);
        self
    }

    pub fn with_max_err_count_per_timespan(mut self, count: usize) -> CircuitBreakerBuilder<S> {
        self.max_err_count_per_timespan = Some(count);
        self
    }

    pub fn with_init_fn(mut self, f: impl DataSrcInitFn<S>) -> CircuitBreakerBuilder<S> {
        self.init_fn = Some(Box::new(f));
        self
    }

    pub fn build(self) -> Result<CircuitBreaker<S>, S::Error> {
        // probably there is a better way to force use all with_* methods on builder
        if self.max_err_count_per_timespan.is_none() {
            panic!("max_err_count_per_timespan is not set");
        }

        if self.max_timespan.is_none() {
            panic!("max_timespan is not set");
        }

        if self.init_fn.is_none() {
            panic!("init_fn is not set");
        }

        let init_fn = self.init_fn.unwrap();

        Ok(CircuitBreaker {
            state: RwLock::new(CBState {
                data_source: Arc::new(init_fn()?),
                err_count: 0,
                first_err_ts: None,
            }),
            max_timespan: self.max_timespan.unwrap(),
            max_err_count_per_timespan: self.max_err_count_per_timespan.unwrap(),
            init_fn,
        })
    }
}

impl<S: FallibleDataSource> CircuitBreaker<S> {
    pub fn builder() -> CircuitBreakerBuilder<S> {
        CircuitBreakerBuilder::new()
    }

    pub fn builder_from_cfg(cfg: &Config) -> CircuitBreakerBuilder<S> {
        Self::builder()
            .with_max_err_count_per_timespan(cfg.max_err_count_per_timespan)
            .with_max_timespan(cfg.max_timespan)
    }

    pub async fn query<T, F, Fut>(&self, query_fn: F) -> Result<T, S::Error>
    where
        F: FnOnce(Arc<S>) -> Fut,
        Fut: Future<Output = Result<T, S::Error>>,
    {
        let state_read_lock = self.state.read().await;
        let result = query_fn(state_read_lock.data_source.clone()).await;

        drop(state_read_lock);

        if let Err(e) = &result {
            if S::is_countable_err(e) {
                let mut state = self.state.write().await;
                state.inc();

                debug!("err count: {}", state.err_count);

                match state.first_err_ts {
                    Some(ts) => {
                        let elapsed = ts.elapsed();

                        if state.err_count <= self.max_err_count_per_timespan {
                            if elapsed > self.max_timespan {
                                state.reset();
                            }
                        } else {
                            return Err(state
                                .data_source
                                .fallback(elapsed.as_millis(), state.err_count));
                        }
                    }
                    None => state.first_err_ts = Some(Instant::now()),
                }
                state.reinit((self.init_fn)()?);
            }
        } else {
            let mut state = self.state.write().await;
            state.reset();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WildErrorGenerator;

    impl WildErrorGenerator {
        fn err(&self) -> Result<(), WildError> {
            Err(WildError::Inner)
        }
    }

    #[derive(Debug)]
    enum WildError {
        Inner,
        CircuitBreakerTriggered,
    }

    impl FallibleDataSource for WildErrorGenerator {
        type Error = WildError;

        fn is_countable_err(err: &Self::Error) -> bool {
            matches!(err, WildError::Inner)
        }

        fn fallback(&self, _elapsed_ms: u128, _err_count: usize) -> Self::Error {
            WildError::CircuitBreakerTriggered
        }
    }

    #[tokio::test]
    async fn circuit_breaker() {
        let cb = CircuitBreaker::builder()
            .with_max_timespan(Duration::from_secs(1))
            .with_max_err_count_per_timespan(2)
            .with_init_fn(|| Ok(WildErrorGenerator))
            .build()
            .unwrap();

        // trigger 2 errors in cb
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            WildError::Inner
        ));

        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            WildError::Inner
        ));

        // reset cb state with successful query
        assert_eq!(cb.query(|_weg| async move { Ok(()) }).await.unwrap(), ());

        // trigger 3 errors in cb (max errors limit exceeded)
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            WildError::Inner
        ));

        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            WildError::Inner
        ));

        // cb fallback
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            WildError::CircuitBreakerTriggered
        ));

        assert_eq!(cb.query(|_weg| async move { Ok(()) }).await.unwrap(), ());
    }
}
