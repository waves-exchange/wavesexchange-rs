#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("CircuitBreaker BuilderError: {0}")]
    BuilderError(String),
}
