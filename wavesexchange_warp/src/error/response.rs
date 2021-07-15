use serde::Serialize;
use std::collections::HashMap;
use warp::{
    http::StatusCode,
    reject::Reject,
    reply::{json, with_status, Reply, Response as WarpResponse},
};

#[derive(Debug, Clone, Serialize)]
pub struct ErrorDetails(HashMap<String, String>);

impl ErrorDetails {
    pub fn single_item(key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let mut hm = HashMap::with_capacity(1);
        hm.insert(key.as_ref().to_owned(), value.as_ref().to_owned());
        Self(hm)
    }

    pub fn add_item(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let mut hm = HashMap::with_capacity(1);
        hm.insert(key.as_ref().to_owned(), value.as_ref().to_owned());
        Self(hm)
    }
}

impl From<HashMap<String, String>> for ErrorDetails {
    fn from(hm: HashMap<String, String>) -> Self {
        Self(hm)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Error {
    pub message: String,
    pub code: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<ErrorDetails>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub errors: Vec<Error>,
}

impl Response {
    pub fn singleton(
        status: StatusCode,
        message: impl Into<String>,
        code: u32,
        details: Option<ErrorDetails>,
    ) -> Self {
        Self {
            errors: vec![Error {
                message: message.into(),
                code: code,
                details: details,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_response_without_details() {
        let res = Response::singleton(StatusCode::NOT_FOUND, "Not Found", 1, None).into_response();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        assert_eq!(format!("{:?}", res.body()), "Body(Full(b\"{\\\"errors\\\":[{\\\"message\\\":\\\"Not Found\\\",\\\"code\\\":1}]}\"))");
    }

    #[test]
    fn should_response_with_details() {
        let mut details = HashMap::new();
        details.insert("parameter_name".to_string(), "key".to_string());

        let res = Response::singleton(
            StatusCode::BAD_REQUEST,
            "Bad Request",
            1,
            Some(ErrorDetails(details)),
        )
        .into_response();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(format!("{:?}", res.body()), "Body(Full(b\"{\\\"errors\\\":[{\\\"message\\\":\\\"Bad Request\\\",\\\"code\\\":1,\\\"details\\\":{\\\"parameter_name\\\":\\\"key\\\"}}]}\"))");
    }
}
