use crate::{ApiResult, BaseApi, HttpClient};
use chrono::{Duration, NaiveDate};

#[derive(Clone, Debug)]
pub struct RateAggregatesApi;

impl BaseApi for RateAggregatesApi {}

impl HttpClient<RateAggregatesApi> {
    pub async fn get(
        &self,
        amount_asset_id: impl AsRef<str>,
        price_asset_id: impl AsRef<str>,
        start_date_inclusive: NaiveDate,
        end_date_inclusive: NaiveDate,
    ) -> ApiResult<dto::RateAggregatesResponse> {
        let timestamp_gte = start_date_inclusive.and_hms(0, 0, 0);
        let timestamp_lt = (end_date_inclusive + Duration::days(1)).and_hms(0, 0, 0);

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
