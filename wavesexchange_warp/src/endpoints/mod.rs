use serde::Serialize;
use std::{fmt::Display, future::Future};
use warp::{
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

pub fn readyz<E, F, Ch>(
    readiness_checker: Ch,
) -> impl Filter<Extract = (HealthcheckReply,), Error = Rejection> + Clone
where
    E: Display,
    F: Future<Output = Result<(), E>> + Send + Sync + 'static,
    Ch: Fn() -> F + Clone + Send + Sync + 'static,
{
    warp::path("readyz").and_then(move || {
        let rc = readiness_checker.clone();
        async move {
            Ok::<_, Rejection>(match rc().await {
                Ok(_) => HealthcheckReply::ok(),
                Err(e) => HealthcheckReply::err(e.to_string()),
            })
        }
    })
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
        let filters = readyz(|| async { Err("not enough racoons") });
        let result = test::request().path("/readyz").reply(&filters).await;
        let result = serde_json::from_slice::<Value>(&result.into_body()).unwrap();
        assert_eq!(result["status"], "not enough racoons");
    }
}
