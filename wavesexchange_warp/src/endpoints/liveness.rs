use serde::Serialize;
use std::{fmt::Debug, future::Future};
use warp::{
    filters::BoxedFilter,
    http::StatusCode,
    reply::{json, with_status, Response},
    Filter, Rejection, Reply,
};

const LIVEZ_URL: &str = "livez";
const READYZ_URL: &str = "readyz";
const STARTZ_URL: &str = "startz";

/// Service readiness status.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Readiness {
    /// Service is fully ready and operating (both `/readyz` and `/livez` returns OK).
    Ready,
    /// Service is temporarily not ready (`/readyz` returns error, but `/livez` is OK).
    NotReady,
    /// Service is completely dead and must ve restarted (both `/readyz` and `/livez` returns error).
    Dead,
}

pub trait Shared: Send + Sync + 'static {}
impl<T> Shared for T where T: Send + Sync + 'static {}

#[derive(Clone)]
pub(crate) struct LivenessReply {
    err: Option<String>,
}

impl LivenessReply {
    pub fn ok() -> Self {
        Self { err: None }
    }

    pub fn err<E: Debug>(msg: E) -> Self {
        Self {
            err: Some(format!("{msg:?}")),
        }
    }
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
}

impl Reply for StatusResponse {
    fn into_response(self) -> Response {
        json(&self).into_response()
    }
}

impl Reply for LivenessReply {
    fn into_response(self) -> Response {
        match self.err {
            Some(e) => with_status(
                StatusResponse { status: e },
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response(),
            None => StatusResponse {
                status: "ok".to_owned(),
            }
            .into_response(),
        }
    }
}

pub(crate) fn livez() -> impl Filter<Extract = (LivenessReply,), Error = Rejection> + Clone {
    warp::path(LIVEZ_URL).map(LivenessReply::ok)
}

pub(crate) fn readyz() -> impl Filter<Extract = (LivenessReply,), Error = Rejection> + Clone {
    warp::path(READYZ_URL).map(LivenessReply::ok)
}

pub(crate) fn startz() -> impl Filter<Extract = (LivenessReply,), Error = Rejection> + Clone {
    warp::path(STARTZ_URL).map(LivenessReply::ok)
}

pub(crate) trait Checkz<E>:
    Filter<Extract = (LivenessReply,), Error = Rejection> + Clone + Shared
where
    E: Debug + Shared,
{
    fn with_checker<F, C>(self, checker: C) -> BoxedFilter<(LivenessReply,)>
    where
        F: Future<Output = Result<(), E>> + Send,
        C: FnOnce() -> F + Clone + Shared,
    {
        Filter::boxed(self.and_then(move |hc: LivenessReply| {
            let checker = checker.clone();
            async move {
                Ok::<_, Rejection>(match (hc.err, checker().await) {
                    (None, Ok(_)) => LivenessReply::ok(),
                    (Some(err), _) => LivenessReply::err(err),
                    (_, Err(err)) => LivenessReply::err(err),
                })
            }
        }))
    }
}

impl<F, E: Debug + Shared> Checkz<E> for F where
    F: Filter<Extract = (LivenessReply,), Error = Rejection> + Clone + Shared
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use warp::test;

    #[tokio::test]
    async fn check_health() {
        let filter = livez();
        let result = test::request().path("/livez").reply(&filter).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn check_readiness() {
        let filter = readyz().with_checker(|| async { Err("not enough racoons") });
        let result = test::request().path("/readyz").reply(&filter).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], format!("{:?}", "not enough racoons"));

        let filter = readyz();
        let result = test::request().path("/readyz").reply(&filter).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn check_send_bounds() {
        #[derive(Clone)]
        struct Str {
            _c: String,
        }

        async fn ctrl_test(_s: Str) -> Result<(), Rejection> {
            Ok(())
        }

        let s = Str {
            _c: String::from("test"),
        };
        let filter = startz().with_checker(|| async { ctrl_test(s).await });
        let result = test::request().path("/startz").reply(&filter).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "ok");
    }
}
