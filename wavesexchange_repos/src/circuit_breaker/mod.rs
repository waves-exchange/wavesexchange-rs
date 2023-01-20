#[cfg(feature = "config")]
mod config;
mod error;

#[cfg(feature = "config")]
pub use config::Config;
pub use error::CBError;
use wavesexchange_log::debug;

use std::{
    future::Future,
    mem::drop,
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub trait DataSrcInitFn<S, E>: FnMut() -> Result<S, E> + Send + Sync + 'static {}

impl<T, S, E> DataSrcInitFn<S, E> for T where T: FnMut() -> Result<S, E> + Send + Sync + 'static {}

/// Count erroneous attempts while quering some data source and perform reinitialization/fallback.
///
/// To use within an object, you must implement `FallibleDataSource` first.
///
/// Example:
/// ```rust
/// use wavesexchange_repos::circuit_breaker::CircuitBreakerBuilder;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     struct Repo;
///
///     #[derive(Debug)]
///     struct RepoError;
///
///     let cb = CircuitBreakerBuilder {
///         max_timespan: Duration::from_secs(1),
///         max_err_count_per_timespan: 5,
///         init_fn: Box::new(|| Ok(Repo))
///     }.build().unwrap();
///
///     cb.query(|src| async move { Err::<(), _>(RepoError) }).await.unwrap_err();
///     cb.query(|src| async move { Ok(()) }).await.unwrap()
///     
///     // see CB test for more verbose example
/// }
/// ```
pub struct CircuitBreaker<S, E> {
    /// Timespan that errors will be counted in.
    /// After it elapsed, error counter will be resetted.
    max_timespan: Duration,

    /// Maximum error count per timespan. Example: 3 errors per 1 sec (max_timespan)
    max_err_count_per_timespan: u16,

    /// Current state of CB
    state: RwLock<CBState<S, E>>,
}

struct CBState<S, E> {
    data_source: Rc<S>,
    err_count: u16,
    first_err_ts: Option<Instant>,

    /// A function that may be called on every fail to reinitialize data source
    init_fn: Box<dyn DataSrcInitFn<S, E>>,
}

impl<S, E> CBState<S, E> {
    fn inc(&mut self) {
        self.err_count += 1;
    }

    fn reset(&mut self) {
        self.err_count = 0;
        self.first_err_ts = None;
    }

    fn reinit(&mut self) -> Result<(), E> {
        self.data_source = Rc::new((self.init_fn)()?);
        Ok(())
    }
}

/// A circuit breaker builder. As all fields are mandatory, use struct creation syntax to init.
/// Example:
/// ```no_compile
/// CircuitBreakerBuilder {
///     max_timespan: Duration::from_secs(1),
///     max_err_count_per_timespan: 5,
///     init_fn: Box::new(|| Ok(Repo))
/// }.build().unwrap()
/// ```
pub struct CircuitBreakerBuilder<S, E> {
    pub max_timespan: Duration,
    pub max_err_count_per_timespan: u16,
    pub init_fn: Box<dyn DataSrcInitFn<S, E>>,
}

impl<S, E> CircuitBreakerBuilder<S, E> {
    pub fn build(self) -> Result<CircuitBreaker<S, E>, E> {
        let Self {
            max_timespan,
            max_err_count_per_timespan,
            mut init_fn,
        } = self;

        Ok(CircuitBreaker {
            state: RwLock::new(CBState {
                data_source: Rc::new(init_fn()?),
                err_count: 0,
                first_err_ts: None,
                init_fn,
            }),
            max_timespan,
            max_err_count_per_timespan,
        })
    }
}

impl<S, E> CircuitBreaker<S, E> {
    /// Query the data source. If succeeded, CB resets internal error counter.
    /// If error returned, counter increases.
    /// If (N > max_err_count_per_timespan) errors appeared, CB is falling back (panic as default).
    /// If not enough errors in a timespan appeared to trigger CB's fallback, error counter will be reset.
    pub async fn query<T, F, Fut>(&self, query_fn: F) -> Result<T, CBError<E>>
    where
        F: FnOnce(Rc<S>) -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let state_read_lock = self.state.read().await;
        let result = query_fn(state_read_lock.data_source.clone()).await;
        let old_err_count = state_read_lock.err_count;

        drop(state_read_lock);

        if let Err(_) = &result {
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
                        return Err(CBError::CircuitBroke {
                            err_count: state.err_count,
                            elapsed,
                        });
                    }
                }
                None => state.first_err_ts = Some(Instant::now()),
            }
            state.reinit().map_err(CBError::Inner)?;
        } else {
            if old_err_count > 0 {
                let mut state = self.state.write().await;
                state.reset();
            }
        }
        result.map_err(CBError::Inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WildErrorGenerator;

    impl WildErrorGenerator {
        fn err(&self) -> Result<(), WildError> {
            Err(WildError)
        }
    }

    #[derive(Debug)]
    struct WildError;

    #[tokio::test]
    async fn circuit_breaker() {
        let cb = CircuitBreakerBuilder {
            max_timespan: Duration::from_secs(1),
            max_err_count_per_timespan: 2,
            init_fn: Box::new(|| Ok(WildErrorGenerator)),
        }
        .build()
        .unwrap();

        // trigger 2 errors in cb
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        // reset cb state with successful query
        assert_eq!(cb.query(|_weg| async move { Ok(()) }).await.unwrap(), ());

        // trigger 3 errors in cb (max errors limit exceeded)
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        // cb fallback
        assert!(matches!(
            cb.query(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::CircuitBroke { .. }
        ));

        assert_eq!(cb.query(|_weg| async move { Ok(()) }).await.unwrap(), ());
    }
}
