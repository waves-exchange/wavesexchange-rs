use serde::Serialize;
use warp::{
    http::StatusCode,
    reject::Reject,
    reply::{json, with_status, Reply, Response as WarpResponse},
};

#[derive(Serialize, Debug, Clone)]
pub struct Error {
    pub message: String,
    pub code: u32,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // details: Option<HashMap<String, T>>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub errors: Vec<Error>,
}

impl Response {
    pub fn singleton(status: StatusCode, message: impl Into<String>, code: u32) -> Self {
        Self {
            errors: vec![Error {
                message: message.into(),
                code: code,
            }],
            status: status,
        }
    }
}

#[derive(Serialize)]
struct ErrorList {
    errors: Vec<Error>,
}

impl Reply for Response {
    fn into_response(self) -> WarpResponse {
        with_status(
            json(&ErrorList {
                errors: self.errors,
            }),
            self.status,
        )
        .into_response()
    }
}

impl Reject for Response {}
