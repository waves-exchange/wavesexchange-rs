use std::fmt::Debug;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum LoaderError<E: Debug> {
    #[error("{0}; check your load_fn, it should return as many values as keys were provided")]
    MissingValues(String),
    #[error("An error encountered: {0}")]
    Other(E),
}
