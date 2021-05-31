#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("InvalidStatePath: {0}")]
    InvalidStatePath(String),
    #[error("UrlParseError: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("InvalidTopic: {0}")]
    InvalidTopic(String),
    #[error("InvalidTransactionType: {0}")]
    InvalidTransactionType(String),
    #[error("InvalidTransactionPath: {0}")]
    InvalidTransactionPath(String),
    #[error("InvalidTransactionQuery: {0}")]
    InvalidTransactionQuery(ErrorQuery),
    #[error("InvalidLeasingPath: {0}")]
    InvalidLeasingPath(String),
}

#[derive(Debug)]
pub struct ErrorQuery(pub Option<String>);

impl std::fmt::Display for ErrorQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            None => write!(f, "None"),
            Some(s) => write!(f, "{}", s.to_owned()),
        }
    }
}
