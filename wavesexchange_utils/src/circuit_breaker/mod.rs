mod error;

pub use error::CBError;
use wavesexchange_log::debug;

use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Count erroneous attempts while quering some data source.
///
/// Example:
/// ```rust
/// use wavesexchange_utils::circuit_breaker::CircuitBreaker;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     struct Repo;
///
///     #[derive(Debug)]
///     struct RepoError;
///
///     let cb = CircuitBreaker::new(
///         Duration::from_secs(1),
///         5,
///         Repo
///     );
///
///     cb.access(|src| async move { Err::<(), _>(RepoError) }).await.unwrap_err();
///     cb.access(|src| async move { Ok::<_, ()>(()) }).await.unwrap()
///
///     // see CB test below for more verbose example
/// }
/// ```
pub struct CircuitBreaker<S> {
    /// Timespan that errors will be counted in.
    /// After it elapsed, error counter will be resetted.
    max_timespan: Duration,

    /// Maximum error count per timespan. Example: 3 errors per 1 sec (max_timespan)
    max_err_count_per_timespan: u16,

    data_source: Arc<S>,

    /// Current state of CB
    state: Mutex<CBState>,
}

impl<S> CircuitBreaker<S> {
    pub fn new(max_timespan: Duration, max_err_count_per_timespan: u16, data_source: S) -> Self {
        Self {
            max_timespan,
            max_err_count_per_timespan,
            data_source: Arc::new(data_source),
            state: Mutex::new(CBState::default()),
        }
    }
}

#[derive(Default)]
struct CBState {
    err_count: u16,
    first_err_ts: Option<Instant>,
}

impl CBState {
    fn inc(&mut self) {
        self.err_count += 1;
    }

    fn reset(&mut self) {
        self.err_count = 0;
        self.first_err_ts = None;
    }
}

impl<S> CircuitBreaker<S> {
    /// Access the data source. If succeeded, CB resets internal error counter.
    /// If error returned, counter is increased.
    /// If (N > max_err_count_per_timespan) errors appeared, CB breaks a circuit,
    /// otherwise error counter will be reset.
    pub async fn access<T, E, F, Fut>(&self, query_fn: F) -> Result<T, CBError<E>>
    where
        F: FnOnce(Arc<S>) -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let result = query_fn(self.data_source.clone()).await;
        self.handle_result(result)
    }

    /// Sync version of `access` method.
    pub fn access_blocking<T, E, F>(&self, query_fn: F) -> Result<T, CBError<E>>
    where
        F: FnOnce(Arc<S>) -> Result<T, E>,
    {
        let result = query_fn(self.data_source.clone());
        self.handle_result(result)
    }

    fn handle_result<T, E>(&self, result: Result<T, E>) -> Result<T, CBError<E>> {
        let mut state = self.state.lock().unwrap();

        if let Err(_) = &result {
            state.inc();

            debug!("CircuitBreaker: err count: {}", state.err_count);

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
        } else {
            if state.err_count > 0 {
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

    const EMPTY_OK: Result<(), ()> = Ok(());

    #[derive(Debug)]
    struct WildError;

    #[tokio::test]
    async fn circuit_breaker() {
        let cb = CircuitBreaker::new(Duration::from_secs(1), 2, WildErrorGenerator);

        // trigger 2 errors in cb
        assert!(matches!(
            cb.access(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        assert!(matches!(
            cb.access(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        // reset cb state with successful query
        assert_eq!(cb.access(|_weg| async move { EMPTY_OK }).await.unwrap(), ());

        // trigger 3 errors in cb (max errors limit exceeded)
        assert!(matches!(
            cb.access(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        assert!(matches!(
            cb.access(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::Inner(WildError)
        ));

        // break circuit
        assert!(matches!(
            cb.access(|weg| async move { weg.err() }).await.unwrap_err(),
            CBError::CircuitBroke { .. }
        ));

        assert_eq!(cb.access(|_weg| async move { EMPTY_OK }).await.unwrap(), ());
    }
}
