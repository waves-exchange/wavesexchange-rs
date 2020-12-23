use serde::de::DeserializeOwned;
use warp::{reject, Filter};

#[derive(Debug)]
pub struct QueryStringDeserializationError(serde_qs::Error);

impl reject::Reject for QueryStringDeserializationError {}

pub fn query<T: DeserializeOwned + Send + 'static>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::filters::query::raw()
        .or_else(|_| async { Result::<_, warp::Rejection>::Ok((String::from(""),)) }) // ::<warp::Rejection>
        .and_then(|q: String| async move {
            serde_qs::from_str::<T>(&q)
                .map_err(|err| reject::custom(QueryStringDeserializationError(err)))
        })
}
