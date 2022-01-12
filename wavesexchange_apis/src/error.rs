use std::sync::Arc;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("HttpRequestError: {1} - {0}")]
    HttpRequestError(Arc<reqwest::Error>, String),

    #[error("InvalidStatus: {1}, status code: {0}")]
    InvalidStatus(reqwest::StatusCode, String),

    #[error("ResponseParseError: {0}")]
    ResponseParseError(String),

    #[error("GrpcError: {0}")]
    GrpcError(#[from] Arc<tonic::transport::Error>),

    #[error("GrpcStatusError: {0}")]
    GrpcStatusError(#[from] Arc<tonic::Status>),
}
