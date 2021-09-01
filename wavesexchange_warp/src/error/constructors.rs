use super::Response;
use warp::http::StatusCode;

mod offsets {
    pub const AUTHENTICATION: u32 = 0;
    pub const AUTHORIZATION: u32 = 1;
    pub const VALIDATION: u32 = 2;
    pub const NOT_IMPLEMENTED: u32 = 3;
    pub const NOT_FOUND: u32 = 4;
    pub const INTERNAL: u32 = 5;
    pub const TIMEOUT: u32 = 6;
    pub const METHOD_NOT_ALLOWED: u32 = 7;
    pub const UNSUPPORTED_MEDIA_TYPE: u32 = 8;
    pub const RESOURCE: u32 = 9;
}

pub fn authentication(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::UNAUTHORIZED,
        "Invalid access token.",
        code_prefix as u32 * 10000 + offsets::AUTHENTICATION * 100,
        None,
    )
}

pub fn authorization(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::FORBIDDEN,
        "Permission denied.",
        code_prefix as u32 * 10000 + offsets::AUTHORIZATION * 100,
        None,
    )
}

pub fn method_not_allowed(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::METHOD_NOT_ALLOWED,
        "Method Not Allowed.",
        code_prefix as u32 * 10000 + offsets::METHOD_NOT_ALLOWED * 100,
        None,
    )
}

pub fn unsuported_media_type(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Unsupported Media Type.",
        code_prefix as u32 * 10000 + offsets::UNSUPPORTED_MEDIA_TYPE * 100,
        None,
    )
}

// todo validation errors after error details are implemented
pub mod validation {
    use std::collections::HashMap;

    use crate::error::response::ErrorDetails;

    use super::{offsets, Response};
    use warp::http::StatusCode;

    pub fn missing_parameter(
        code_prefix: u16,
        details: Option<HashMap<String, String>>,
    ) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Missing required parameter.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100,
            details.map(|details| ErrorDetails::from(details)),
        )
    }

    pub fn invalid_parameter(
        code_prefix: u16,
        details: Option<HashMap<String, String>>,
    ) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Invalid parameter value.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100 + 1,
            details.map(|details| ErrorDetails::from(details)),
        )
    }

    pub fn missing_header(code_prefix: u16, details: Option<HashMap<String, String>>) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Missing required header.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100 + 2,
            details.map(|details| ErrorDetails::from(details)),
        )
    }

    pub fn invalid_header(code_prefix: u16, details: Option<HashMap<String, String>>) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Invalid header value.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100 + 3,
            details.map(|details| ErrorDetails::from(details)),
        )
    }

    pub fn body_deserialization(
        code_prefix: u16,
        details: Option<HashMap<String, String>>,
    ) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Body deserialization error.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100 + 4,
            details.map(|details| ErrorDetails::from(details)),
        )
    }

    pub fn query_deserialization(
        code_prefix: u16,
        details: Option<HashMap<String, String>>,
    ) -> Response {
        Response::singleton(
            StatusCode::BAD_REQUEST,
            "Query deserialization error.",
            code_prefix as u32 * 10000 + offsets::VALIDATION * 100 + 5,
            details.map(|details| ErrorDetails::from(details)),
        )
    }
}

pub fn not_implemented(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::NOT_IMPLEMENTED,
        "Not implemented.",
        code_prefix as u32 * 10000 + offsets::NOT_IMPLEMENTED * 100,
        None,
    )
}

pub fn requests_limit(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::TOO_MANY_REQUESTS,
        "Requests limit reached.",
        code_prefix as u32 * 10000 + offsets::RESOURCE * 100,
        None,
    )
}

pub fn not_found(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::NOT_FOUND,
        "Not found.",
        code_prefix as u32 * 10000 + offsets::NOT_FOUND * 100,
        None,
    )
}

pub fn internal(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::INTERNAL_SERVER_ERROR,
        internal::MESSAGE,
        code_prefix as u32 * 10000 + offsets::INTERNAL * 100,
        None,
    )
}

// todo subcodes after error details
pub mod internal {
    //     use super::Response;
    //     use warp::http::StatusCode;

    pub const MESSAGE: &str = "Internal server error";
    //     pub const CODE_OFFSET: u32 = 500;

    //     // todo subcode in details
    //     fn database(code_prefix: u16) -> Response {
    //         Response::singleton(
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             MESSAGE,
    //             code_prefix as u32 * 10000 + CODE_OFFSET,
    //         )
    //     }

    //     fn upstream(code_prefix: u16) -> Response {
    //         Response::singleton(
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             MESSAGE,
    //             code_prefix as u32 * 10000 + CODE_OFFSET,
    //         )
    //     }
}

pub fn timeout(code_prefix: u16) -> Response {
    Response::singleton(
        StatusCode::GATEWAY_TIMEOUT,
        "Timed out.",
        code_prefix as u32 * 10000 + offsets::TIMEOUT * 100,
        None,
    )
}
