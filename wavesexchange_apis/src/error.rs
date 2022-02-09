use reqwest::{Error as ReqError, Response};
use std::sync::Arc;

pub type ApiResult<T> = Result<T, Error>;

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

pub async fn invalid_status(resp: Response, req_info: impl Into<String>) -> Error {
    let status = resp.status();
    let url = resp.url().to_string();
    let body = resp.text().await.unwrap_or_else(|_| "".to_owned());
    let req_info = req_info.into();
    Error::InvalidStatus(
        status,
        format!(r#"Upstream API error on request "{req_info}", url: {url}, body: "{body}""#),
    )
}

pub fn request_failed(err: ReqError, req_info: impl Into<String>) -> Error {
    let req_info = req_info.into();
    Error::HttpRequestError(Arc::new(err), format!("Request \"{req_info}\" failed"))
}

pub fn json_error(err: &str, req_info: impl Into<String>, resp_body: &str) -> Error {
    let req_info = req_info.into();
    Error::ResponseParseError(format!(
        r#"Failed to parse json on request "{req_info}": {err}; body: "{resp_body}""#
    ))
}
