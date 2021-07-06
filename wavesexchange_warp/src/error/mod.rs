mod constructors;
mod response;

// reexport
pub use constructors::*;
pub use response::{Error, Response};

use futures::future::Ready;
use std::{convert::Infallible, sync::Arc};
use warp::{
    reject::{InvalidHeader, MissingHeader, Reject},
    Rejection, Reply,
};

use crate::query::QueryStringDeserializationError;

pub fn handler<E: Reject>(
    error_code_prefix: u16,
    handle: impl Fn(&E) -> Response,
) -> impl Fn(Rejection) -> Ready<Result<warp::reply::Response, Infallible>> + Clone {
    let handler = Arc::new(handle);

    move |r: Rejection| {
        let resp: Response;

        if r.is_not_found() {
            resp = not_found(error_code_prefix.clone());
        } else if let Some(e) = r.find::<warp::filters::body::BodyDeserializeError>() {
            resp = validation::body_deserialization(error_code_prefix.clone(), Some(e.to_string()));
        } else if let Some(e) = r.find::<QueryStringDeserializationError>() {
            resp = validation::invalid_parameter(error_code_prefix.clone(), e.0.to_string());
        } else if let Some(e) = r.find::<InvalidHeader>() {
            resp = validation::invalid_header(error_code_prefix.clone(), e.name());
        } else if let Some(e) = r.find::<MissingHeader>() {
            resp = validation::missing_header(error_code_prefix.clone(), e.name());
        } else if let Some(crate_error) = r.find::<E>() {
            resp = handler(crate_error);
        } else {
            resp = internal(error_code_prefix.clone());
        }

        futures::future::ok(resp.into_response())
    }
}

pub fn error_handler_with_serde_qs<'a>(
    error_code_prefix: u16,
    error_handler: impl Fn(Rejection) -> futures::future::Ready<Result<warp::reply::Response, Infallible>>
        + 'a,
) -> impl Fn(Rejection) -> futures::future::Ready<Result<warp::reply::Response, Infallible>> + 'a {
    move |rej: Rejection| {
        if let Some(err) = rej.find::<serde_qs::Error>() {
            futures::future::ready(Ok(
                validation::invalid_parameter(error_code_prefix, err.to_string()).into_response()
            ))
        } else {
            error_handler(rej)
        }
    }
}
