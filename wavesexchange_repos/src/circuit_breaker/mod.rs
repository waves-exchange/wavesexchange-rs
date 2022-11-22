/*
послать запрос к удалённому ресурсу
если пришла ошибка соединения, то зафиксировать и вернуть ошибку в Result
если разрывы соединений продолжаются, выкинуть панику (или вызвать соотв обработчик)
разрывы соединений:
    разные бд (pg, redis), разные пулы (bb8, deadpool, r2d2), одиночный запрос, возможность расширения
*/
pub mod config;
pub mod error;
pub mod impls;

pub use config::Config;
pub use error::Error;
pub use impls::*;

use std::{
    future::Future,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

pub struct CircuitBreaker<S: FallibleDataSource> {
    data_source: S,
    err_count: usize, // current errors count
    first_err_ts: Option<Instant>,
    max_timespan: Duration, // максимальный временной промежуток, в котором будут считаться ошибки
    max_err_count_per_timespan: NonZeroUsize,
    init_fn: Box<dyn Fn() -> S>,
}

pub struct CircuitBreakerBuilder<S: FallibleDataSource> {
    max_timespan: Option<Duration>,
    max_err_count_per_timespan: Option<NonZeroUsize>,
    init_fn: Option<Box<dyn Fn() -> S>>,
}

impl<S: FallibleDataSource> CircuitBreakerBuilder<S> {
    pub fn new() -> CircuitBreakerBuilder<S> {
        CircuitBreakerBuilder {
            max_timespan: None,
            max_err_count_per_timespan: None,
            init_fn: None,
        }
    }

    pub fn max_timespan(mut self, ts: Duration) -> CircuitBreakerBuilder<S> {
        self.max_timespan = Some(ts);
        self
    }

    pub fn max_err_count_per_timespan(mut self, count: usize) -> CircuitBreakerBuilder<S> {
        self.max_err_count_per_timespan = NonZeroUsize::new(count);
        self
    }

    pub fn init_fn(mut self, f: impl Fn() -> S + 'static) -> CircuitBreakerBuilder<S> {
        self.init_fn = Some(Box::new(f));
        self
    }

    pub fn build(self) -> Result<CircuitBreaker<S>, Error> {
        let build_err = |s: &str| Err(Error::BuilderError(s.to_string()));

        if self.max_err_count_per_timespan.is_none() {
            return build_err("max_err_count_per_timespan is not set");
        }

        if self.max_timespan.is_none() {
            return build_err("max_timespan is not set");
        }

        if self.init_fn.is_none() {
            return build_err("init_fn is not set");
        }

        let init_fn = self.init_fn.unwrap();

        Ok(CircuitBreaker {
            data_source: init_fn(),
            err_count: 0,
            first_err_ts: None,
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
            .max_err_count_per_timespan(cfg.max_err_count_per_timespan)
            .max_timespan(cfg.max_timespan)
    }

    pub async fn query<T, F, Fut>(&mut self, query_fn: F) -> Result<T, S::Error>
    where
        F: Fn(&S) -> Fut,
        Fut: Future<Output = Result<T, S::Error>>,
    {
        let result = query_fn(&self.data_source).await;

        if let Err(e) = &result {
            if S::is_countable_err(e) {
                self.err_count += 1;
                match self.first_err_ts {
                    Some(ts) if ts.elapsed() <= self.max_timespan => {
                        if self.err_count > self.max_err_count_per_timespan.get() {
                            return Err(self.data_source.fallback());
                        }
                    }
                    None => self.first_err_ts = Some(Instant::now()),
                    _ => {}
                }
                if S::REINIT_ON_FAIL {
                    self.data_source = (self.init_fn)();
                }
            }
        } else {
            self.err_count = 0;
            self.first_err_ts = None;
        }
        result
    }
}

pub trait FallibleDataSource {
    const REINIT_ON_FAIL: bool;
    type Error;

    fn is_countable_err(err: &Self::Error) -> bool;

    fn fallback(&self) -> Self::Error {
        panic!("Я ГОВОРЮ НЕ БЫЛО РАЗРЫВОВ СВЯЗИ! С НОЯБРЯ ПРОШЛОГО ГОДА! А СЕЙЧАС ЦЕЛЫХ ЧЕТЫРЕ РАЗРЫВА БЫЛО!")
    }
}
