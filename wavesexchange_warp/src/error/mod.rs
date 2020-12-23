mod constructors;
mod response;

// reexport
pub use constructors::*;
pub use response::{Error, Response};

use futures::future::Ready;
use std::{convert::Infallible, sync::Arc};
use warp::{
    reject::InvalidHeader,
    reject::{MissingHeader, Reject},
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
        } else if let Some(_) = r.find::<warp::filters::body::BodyDeserializeError>() {
            resp = validation::body_deserialization(error_code_prefix.clone());
        // todo header name
        } else if let Some(_) = r.find::<QueryStringDeserializationError>() {
            // todo proper qs deserialization error
            resp = validation::invalid_parameter(error_code_prefix.clone());
        } else if let Some(_) = r.find::<InvalidHeader>() {
            resp = validation::invalid_header(error_code_prefix.clone());
        } else if let Some(_) = r.find::<MissingHeader>() {
            resp = validation::missing_header(error_code_prefix.clone());
        } else if let Some(crate_error) = r.find::<E>() {
            resp = handler(crate_error);
        } else {
            resp = internal(error_code_prefix.clone());
        }

        futures::future::ok(resp.into_response())
    }
}
