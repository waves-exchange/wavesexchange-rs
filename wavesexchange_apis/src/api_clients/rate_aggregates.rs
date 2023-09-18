use crate::{ApiResult, BaseApi, HttpClient};
use chrono::{Duration, NaiveDate};
use futures::future::try_join_all;
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct RateAggregates;

impl BaseApi for RateAggregates {}

impl HttpClient<RateAggregates> {
    /// Get rate aggregates for a single asset pair.
    pub async fn get(
        &self,
        amount_asset_id: impl AsRef<str>,
        price_asset_id: impl AsRef<str>,
        start_date_inclusive: NaiveDate,
        end_date_inclusive: NaiveDate,
    ) -> ApiResult<dto::RateAggregatesResponse> {
        let timestamp_gte = start_date_inclusive
            .and_hms_opt(0, 0, 0)
            .expect("invalid time");
        let timestamp_lt = (end_date_inclusive + Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .expect("invalid time");

        let request_url = format!(
            "rate_aggregates?pairs[]={}/{}&timestamp__gte={:?}&timestamp__lt={:?}",
            amount_asset_id.as_ref(),
            price_asset_id.as_ref(),
            timestamp_gte,
            timestamp_lt
        );

        self.create_req_handler(self.http_get(&request_url), "rate_aggregates::get")
            .execute()
            .await
    }

    const MAX_PAIRS_PER_REQUEST: usize = 32;

    /// Get rate aggregates for multiple asset pairs.
    /// Maximum number of pairs is 32 (limited by GET URL length).
    pub async fn mget(
        &self,
        asset_pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        start_date_inclusive: NaiveDate,
        end_date_inclusive: NaiveDate,
    ) -> ApiResult<dto::RateAggregatesResponse> {
        let asset_pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect_vec();

        // This API has limit on number of pairs,
        // so if exceeded - split long request into smaller chunks
        if asset_pairs.len() > Self::MAX_PAIRS_PER_REQUEST {
            let chunks = asset_pairs
                .chunks(Self::MAX_PAIRS_PER_REQUEST)
                .map(|chunk| {
                    self.mget_impl(chunk.to_vec(), start_date_inclusive, end_date_inclusive)
                });
            let res_chunks = try_join_all(chunks).await?;
            let res = res_chunks
                .into_iter()
                .reduce(|mut acc, e| {
                    acc.items.extend(e.items.into_iter());
                    acc
                })
                .expect("response chunks lost");
            return Ok(res);
        }

        self.mget_impl(asset_pairs, start_date_inclusive, end_date_inclusive)
            .await
    }

    async fn mget_impl(
        &self,
        asset_pairs: Vec<(String, String)>,
        start_date_inclusive: NaiveDate,
        end_date_inclusive: NaiveDate,
    ) -> ApiResult<dto::RateAggregatesResponse> {
        assert!(
            asset_pairs.len() <= Self::MAX_PAIRS_PER_REQUEST,
            "internal error - bad split"
        );

        let qs_pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| format!("pairs[]={}/{}", a, b))
            .join("&");

        let timestamp_gte = start_date_inclusive
            .and_hms_opt(0, 0, 0)
            .expect("invalid time");
        let timestamp_lt = (end_date_inclusive + Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .expect("invalid time");

        let request_url = format!(
            "rate_aggregates?{}&timestamp__gte={:?}&timestamp__lt={:?}",
            qs_pairs, timestamp_gte, timestamp_lt
        );

        self.create_req_handler(self.http_get(&request_url), "rate_aggregates::mget")
            .execute()
            .await
    }
}

pub mod dto {
    use chrono::NaiveDateTime;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct RateAggregatesResponse {
        pub items: Vec<RateAggregates>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct RateAggregates {
        pub interval_start: NaiveDateTime,
        pub interval_end: NaiveDateTime,
        pub aggregates: Vec<AggregatesOuter>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AggregatesOuter {
        pub pair: String,
        pub rates: Aggregates,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Aggregates {
        pub open: Option<f64>,
        pub close: Option<f64>,
        pub high: Option<f64>,
        pub low: Option<f64>,
        pub average: Option<f64>,
    }
}
