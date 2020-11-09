mod constructors;
mod response;

// reexport
pub use constructors::*;
pub use response::{Error, Response};

use std::{convert::Infallible, sync::Arc};
use futures::future::Ready;
use warp::{reject::Reject, Rejection, Reply};

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
        } else if let Some(crate_error) = r.find::<E>() {
            resp = handler(crate_error);
        } else {
            resp = internal(error_code_prefix.clone());
        }

        futures::future::ok(resp.into_response())
    }
}
