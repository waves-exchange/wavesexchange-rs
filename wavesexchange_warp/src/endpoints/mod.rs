pub mod metrics;

use serde::Serialize;
use std::fmt::Debug;
use std::future::Future;
use warp::{
    filters::BoxedFilter,
    http::StatusCode,
    reply::{json, with_status, Response},
    Filter, Rejection, Reply,
};

pub const LIVEZ_URL: &str = "livez";
pub const READYZ_URL: &str = "readyz";
pub const STARTZ_URL: &str = "startz";

pub trait Shared: Send + Sync + 'static {}
impl<T> Shared for T where T: Send + Sync + 'static {}

#[derive(Clone)]
pub struct HealthcheckReply {
    err: Option<String>,
}

impl HealthcheckReply {
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

impl Reply for HealthcheckReply {
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

pub fn livez() -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone {
    warp::path(LIVEZ_URL).map(HealthcheckReply::ok)
}

pub fn readyz() -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone {
    warp::path(READYZ_URL).map(HealthcheckReply::ok)
}

pub fn startz() -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone {
    warp::path(STARTZ_URL).map(HealthcheckReply::ok)
}

pub trait Checkz<E>:
    Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone + Shared
where
    E: Debug + Shared,
{
    fn with_checker<F, C>(self, checker: C) -> BoxedFilter<(HealthcheckReply,)>
    where
        F: Future<Output = Result<(), E>> + Send,
        C: FnOnce() -> F + Clone + Shared,
    {
        Filter::boxed(self.and_then(move |hc: HealthcheckReply| {
            let checker = checker.clone();
            async move {
                Ok::<_, Rejection>(match (hc.err, checker().await) {
                    (None, Ok(_)) => HealthcheckReply::ok(),
                    (Some(err), _) => HealthcheckReply::err(err),
                    (_, Err(err)) => HealthcheckReply::err(err),
                })
            }
        }))
    }
}

impl<F, E: Debug + Shared> Checkz<E> for F where
    F: Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone + Shared
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
        let request = test::request().path("/readyz");
        let filters = readyz().with_checker(|| async { Err("not enough racoons") });
        let result = request.reply(&filters).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], format!("{:?}", "not enough racoons"));

        let request = test::request().path("/readyz");
        let filters = readyz();
        let result = request.reply(&filters).await;
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

        let request = test::request().path("/startz");

        let s = Str {
            _c: String::from("test"),
        };
        let filters = startz().with_checker(|| async { ctrl_test(s).await });
        let result = request.reply(&filters).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "ok");
    }
}
