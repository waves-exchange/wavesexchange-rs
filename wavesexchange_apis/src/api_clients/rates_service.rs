use crate::{ApiResult, BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use cached::proc_macro::cached;
use chrono::Duration;
use itertools::Itertools;
use reqwest::StatusCode;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::future::Future;

#[derive(Clone, Debug)]
pub struct RatesSvcApi;

impl BaseApi for RatesSvcApi {}

impl HttpClient<RatesSvcApi> {
    pub async fn rates<S: Into<String>>(
        &self,
        asset_pairs: impl IntoIterator<Item = (S, S)> + Send,
    ) -> ApiResult<HashMap<(String, String), Rate>> {
        let pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| format!("{}/{}", a.into(), b.into()))
            .collect::<Vec<_>>();

        let body = dto::RatesRequest { pairs };

        let resp: dto::RatesResponse = self
            .create_req_handler(self.post("rates").json(&body), "rates::rates")
            .execute()
            .await?;

        let res = resp
            .data
            .into_iter()
            .filter_map(|rate| {
                let (a, p) = rate.pair.splitn(2, '/').collect_tuple()?;
                let key = (a.to_owned(), p.to_owned());
                let rate = rate.into();
                Some((key, rate))
            })
            .collect::<HashMap<_, _>>();

        Ok(res)
    }

    pub async fn rates_to_same_asset(
        &self,
        amount_assets: impl IntoIterator<Item = impl Into<String>> + Send,
        price_asset: impl AsRef<str> + Send,
    ) -> ApiResult<HashMap<String, Rate>> {
        let price_asset = price_asset.as_ref();

        let pairs = amount_assets
            .into_iter()
            .map(|amount_asset| format!("{}/{}", amount_asset.into(), price_asset))
            .collect::<Vec<_>>();

        let body = dto::RatesRequest { pairs };

        let resp: dto::RatesResponse = self
            .create_req_handler(self.post("rates").json(&body), "rates::rates_to_same_asset")
            .execute()
            .await?;

        let res = resp
            .data
            .into_iter()
            .filter_map(|rate| {
                let (a, p) = rate.pair.splitn(2, '/').collect_tuple()?;
                debug_assert_eq!(p, price_asset);
                let key = a.to_owned();
                let rate = rate.into();
                Some((key, rate))
            })
            .collect::<HashMap<_, _>>();

        Ok(res)
    }

    pub async fn exchange_rates(
        &self,
        assets: impl IntoIterator<Item = impl Into<String>> + Send,
        to_asset: impl AsRef<str> + Send,
    ) -> ApiResult<Option<dto::RatesResponse>> {
        let to_asset = to_asset.as_ref();
        let mut pairs: Vec<_> = vec![];
        pairs.push(format!("{}/{}", "WAVES", to_asset));

        assets
            .into_iter()
            .map(|a| {
                pairs.push(format!("{}/{}", a.into(), to_asset));
            })
            .count();

        let query = dto::RatesRequest { pairs };
        self.create_req_handler(self.post("rates/").json(&query), "rates::exchange_rates")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn get_rates_per_day(
        &self,
        amount_asset_id: impl AsRef<str>,
        price_asset_id: impl AsRef<str>,
        days: Duration,
    ) -> ApiResult<Vec<BigDecimal>> {
        let req = dto::RatesRequest {
            pairs: vec![format!(
                "{}/{}",
                amount_asset_id.as_ref(),
                price_asset_id.as_ref()
            )],
        };

        let cache_key = format!(
            "{}/{}",
            amount_asset_id.as_ref().to_owned(),
            price_asset_id.as_ref().to_owned()
        );
        let response: dto::RatesResponse = cached_rates_response(
            cache_key,
            self.create_req_handler(self.post("rates").json(&req), "rates::get_rates_per_day")
                .execute(),
        )
        .await?;

        // NB: current rates service implementation can return only the current rate
        response
            .data
            .into_iter()
            .next()
            .map(|rate| vec![rate.data.rate; days.num_days() as usize])
            .ok_or(Error::ResponseParseError("no rates found".to_string()))
    }

    pub async fn mget_rates_per_day(
        &self,
        pairs: impl IntoIterator<Item = impl Into<String> + Debug> + Debug,
        days: Duration,
    ) -> ApiResult<Vec<Vec<BigDecimal>>> {
        let cache_key = format!("{pairs:?}");
        let req = dto::RatesRequest {
            pairs: pairs.into_iter().map(Into::into).collect(),
        };

        let response: dto::RatesResponse = cached_rates_response(
            cache_key,
            self.create_req_handler(self.post("rates").json(&req), "rates::mget_rates_per_day")
                .execute(),
        )
        .await?;

        // NB: current rates service implementation can return only the current rate
        Ok(response
            .data
            .into_iter()
            .map(|rate| vec![rate.data.rate; days.num_days() as usize])
            .collect())
    }
}

#[cached(key = "String", convert = r#"{ _key.clone() }"#, result, time = 600)]
async fn cached_rates_response(
    _key: String,
    f: impl Future<Output = ApiResult<dto::RatesResponse>>,
) -> ApiResult<dto::RatesResponse> {
    f.await
}

#[derive(Clone, Debug)]
pub struct Rate {
    pub heuristics: HashSet<String>,
    pub rate: BigDecimal,
    pub heuristic_rate: Option<BigDecimal>,
    pub exchange_rate: Option<BigDecimal>,
}

impl From<dto::Rate> for Rate {
    fn from(value: dto::Rate) -> Self {
        Rate {
            heuristics: HashSet::from_iter(value.heuristics.into_iter()),
            rate: value.data.rate,
            heuristic_rate: value.data.heuristic,
            exchange_rate: value.data.exchange,
        }
    }
}

pub mod dto {
    use bigdecimal::BigDecimal;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Clone)]
    pub struct RatesResponse {
        pub data: Vec<Rate>,
    }

    #[derive(Deserialize, Clone)]
    pub struct Rate {
        pub pair: String,
        pub heuristics: Vec<String>,
        pub data: RateData,
    }

    #[derive(Deserialize, Clone)]
    pub struct RateData {
        pub rate: BigDecimal,
        pub heuristic: Option<BigDecimal>,
        pub exchange: Option<BigDecimal>,
    }

    #[derive(Debug, Serialize)]
    pub struct RatesRequest {
        pub pairs: Vec<String>,
    }
}
