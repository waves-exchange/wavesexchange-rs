/*
послать запрос к удалённому ресурсу
если пришла ошибка соединения, то зафиксировать и вернуть ошибку в Result
если разрывы соединений продолжаются, выкинуть панику (или вызвать соотв обработчик)
разрывы соединений:
    разные бд (pg, redis), разные пулы (bb8, deadpool, r2d2), одиночный запрос, возможность расширения
*/
use derive_builder::Builder;
use std::{
    future::Future,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

#[derive(Debug, Builder)]
#[builder(pattern = "owned")]
pub struct CircuitBreaker<Repo: FallibleRepo> {
    repo: Repo,
    #[builder(setter(skip))]
    err_count: usize, // current errors count
    #[builder(setter(skip))]
    first_err_ts: Option<Instant>,
    max_timespan: Duration, // максимальный временной промежуток, в котором будут считаться ошибки
    #[builder(setter(custom))]
    max_err_count_per_timespan: NonZeroUsize,
}

impl<Repo: FallibleRepo> CircuitBreakerBuilder<Repo> {
    pub fn max_err_count_per_timespan(mut self, ts: usize) -> CircuitBreakerBuilder<Repo> {
        self.max_err_count_per_timespan = NonZeroUsize::new(ts);
        self
    }
}

impl<Repo: FallibleRepo> CircuitBreaker<Repo> {
    pub fn builder() -> CircuitBreakerBuilder<Repo> {
        CircuitBreakerBuilder::default()
    }

    pub async fn query<T, F, Fut>(&mut self, query_fn: F) -> Result<T, Repo::Error>
    where
        F: Fn(&Repo) -> Fut,
        Fut: Future<Output = Result<T, Repo::Error>>,
    {
        let result = query_fn(&self.repo).await;

        if let Err(e) = &result {
            if Repo::is_countable_err(e) {
                self.err_count += 1;
                match self.first_err_ts {
                    Some(ts) if ts.elapsed() <= self.max_timespan => {
                        if self.err_count > self.max_err_count_per_timespan.get() {
                            return Err(self.repo.fallback());
                        }
                    }
                    None => self.first_err_ts = Some(Instant::now()),
                    _ => {}
                }
            }
        } else {
            self.err_count = 0;
            self.first_err_ts = None;
        }
        result
    }
}

#[async_trait]
pub trait FallibleRepo {
    type Error;

    async fn init<C>(cfg: &C) -> Self;

    fn is_countable_err(err: &Self::Error) -> bool;

    fn fallback(&self) -> Self::Error {
        panic!("Я ГОВОРЮ НЕ БЫЛО РАЗРЫВОВ СВЯЗИ! С НОЯБРЯ ПРОШЛОГО ГОДА! А СЕЙЧАС ЦЕЛЫХ ЧЕТЫРЕ РАЗРЫВА БЫЛО!")
    }
}

pub mod impls {
    use super::*;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("CircuitBreakerBuilderError: {0}")]
    BuilderError(String),
}
