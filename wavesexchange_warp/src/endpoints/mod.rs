use serde::Serialize;
use std::{fmt::Display, future::Future};
use warp::{
    filters::BoxedFilter,
    http::StatusCode,
    reply::{json, with_status, Response},
    Filter, Rejection, Reply,
};

#[derive(Clone)]
pub struct HealthcheckReply {
    err: Option<String>,
}

impl HealthcheckReply {
    pub fn ok() -> Self {
        Self { err: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            err: Some(msg.into()),
        }
    }
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
}

impl Reply for HealthcheckReply {
    fn into_response(self) -> Response {
        match self.err {
            Some(e) => with_status(
                json(&StatusResponse { status: e }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response(),
            None => json(&StatusResponse {
                status: "ok".to_owned(),
            })
            .into_response(),
        }
    }
}

pub fn livez() -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone {
    warp::path("livez").map(HealthcheckReply::ok)
}

pub fn readyz() -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone {
    warp::path("readyz").map(HealthcheckReply::ok)
}

pub trait Checkz:
    Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone + Send + Sync + 'static
{
    fn with_checker<E, F, C>(self, checker: C) -> BoxedFilter<(HealthcheckReply,)>
    where
        E: Display,
        F: Future<Output = Result<(), E>> + Send + Sync + 'static,
        C: Fn() -> F + Clone + Send + Sync + 'static,
    {
        Filter::boxed(self.and_then(move |hc: HealthcheckReply| {
            let ch = checker.clone();
            async move {
                Ok::<_, Rejection>(match (hc.err, ch().await) {
                    (None, Ok(_)) => HealthcheckReply::ok(),
                    (Some(err), _) => HealthcheckReply::err(err),
                    (_, Err(err)) => HealthcheckReply::err(err.to_string()),
                })
            }
        }))
    }
}

impl<F> Checkz for F where
    F: Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone + Send + Sync + 'static
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
        assert_eq!(result["status"], "not enough racoons");

        let request = test::request().path("/readyz");
        let filters = readyz();
        let result = request.reply(&filters).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "ok");
    }
}
