use crate::{ApiResult, BaseApi, HttpClient};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct ExchangesService;

impl BaseApi for ExchangesService {}

impl HttpClient<ExchangesService> {
    pub async fn matcher_exchange_aggregates(
        &self,
        timestamp_gte: NaiveDateTime,
        timestamp_lt: NaiveDateTime,
    ) -> ApiResult<MatcherExchangeAggregatesResponse> {
        let qs = serde_qs::to_string(&MatcherExchangeAggregatesRequest {
            timestamp_gte: timestamp_gte.and_utc(),
            timestamp_lt: timestamp_lt.and_utc(),
        })
        .unwrap();

        let res = self
            .create_req_handler(
                self.http_get(format!("matcher_exchange_aggregates?{}", qs)),
                "exchanges::matcher_exchange_aggregates",
            )
            .execute()
            .await?;

        Ok(res)
    }
}

#[derive(Debug, Serialize)]
struct MatcherExchangeAggregatesRequest {
    #[serde(rename = "block_timestamp__gte")]
    timestamp_gte: DateTime<Utc>,
    #[serde(rename = "block_timestamp__lt")]
    timestamp_lt: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct MatcherExchangeAggregatesResponse {
    #[serde(rename = "items")]
    pub data: Vec<MatcherExchangeAggregatesItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatcherExchangeAggregatesItem {
    pub amount_asset: String,
    pub price_asset: String,
    pub interval_start: NaiveDateTime,
    pub interval_end: NaiveDateTime,
    #[serde(deserialize_with = "f64_str")]
    pub total_amount: f64,
    #[serde(deserialize_with = "f64_str")]
    pub price_open: f64,
    #[serde(deserialize_with = "f64_str")]
    pub price_close: f64,
    #[serde(deserialize_with = "f64_str")]
    pub price_high: f64,
    #[serde(deserialize_with = "f64_str")]
    pub price_low: f64,
    #[serde(deserialize_with = "f64_str")]
    pub price_avg: f64,
}

fn f64_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    f64::from_str(&buf).map_err(serde::de::Error::custom)
}
