use cached::proc_macro::cached;
use chrono::{Duration, NaiveDate};
use std::future::Future;

use crate::{ApiResult, BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct RateAggregatesApi;

impl BaseApi for RateAggregatesApi {}

impl HttpClient<RateAggregatesApi> {
    pub async fn get_aggregates(
        &self,
        amount_asset_id: impl AsRef<str>,
        price_asset_id: impl AsRef<str>,
        start_date_inclusive: NaiveDate,
        end_date_inclusive: NaiveDate,
    ) -> ApiResult<Vec<(NaiveDate, Option<HighLow>)>> {
        let timestamp_gte = start_date_inclusive.and_hms(0, 0, 0);
        let timestamp_lt = (end_date_inclusive + Duration::days(1)).and_hms(0, 0, 0);

        let request_url = format!(
            "rate_aggregates?pairs[]={}/{}&timestamp__gte={:?}&timestamp__lt={:?}",
            amount_asset_id.as_ref(),
            price_asset_id.as_ref(),
            timestamp_gte,
            timestamp_lt
        );

        let cache_key = format!(
            "{}/{}@{:?}—{:?}",
            amount_asset_id.as_ref(),
            price_asset_id.as_ref(),
            start_date_inclusive,
            end_date_inclusive
        );
        let resp: dto::RateAggregatesResponse = cached_get(
            cache_key,
            self.create_req_handler(self.get(&request_url), "rate_aggregates::get")
                .execute(),
        )
        .await?;

        let items: Vec<(NaiveDate, Option<HighLow>)> = resp
            .items
            .into_iter()
            .map(|ra| {
                let date = ra.interval_start.date();
                let rate_high_low = ra
                    .aggregates
                    .first()
                    .and_then(|aggr| aggr.rates.high.zip(aggr.rates.low))
                    .map(|(high, low)| HighLow { high, low });

                (date, rate_high_low)
            })
            .collect();

        Ok(items)
    }
}

#[cached(key = "String", convert = r#"{ _key.clone() }"#, result, time = 600)]
async fn cached_get(
    _key: String,
    f: impl Future<Output = ApiResult<dto::RateAggregatesResponse>>,
) -> ApiResult<dto::RateAggregatesResponse> {
    f.await
}

#[derive(Debug, Clone)]
pub struct HighLow {
    pub high: f64,
    pub low: f64,
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
        pub aggregates: Vec<AggregatesOuter>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AggregatesOuter {
        pub rates: Aggregates,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Aggregates {
        pub high: Option<f64>,
        pub low: Option<f64>,
    }
}
