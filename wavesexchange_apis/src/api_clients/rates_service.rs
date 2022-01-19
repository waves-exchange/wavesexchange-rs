use crate::{BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use itertools::Itertools;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use wavesexchange_log::debug;

#[derive(Clone)]
pub struct RatesSvcApi;

impl BaseApi for RatesSvcApi {}

impl HttpClient<RatesSvcApi> {
    pub async fn rates(
        &self,
        asset_pairs: impl IntoIterator<Item = (&str, &str)> + Send,
    ) -> Result<HashMap<(String, String), Rate>, Error> {
        let pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| format!("{}/{}", a, b))
            .collect::<Vec<_>>();

        let resp = self.query_rates(pairs).await?;

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
        amount_assets: impl IntoIterator<Item = &str> + Send,
        price_asset: impl AsRef<str> + Send,
    ) -> Result<HashMap<String, Rate>, Error> {
        let price_asset = price_asset.as_ref();

        let pairs = amount_assets
            .into_iter()
            .map(|amount_asset| format!("{}/{}", amount_asset, price_asset))
            .collect::<Vec<_>>();

        let resp = self.query_rates(pairs).await?;

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

    async fn query_rates(&self, pairs: Vec<String>) -> Result<dto::RatesResponse, Error> {
        let url = "rates";
        let body = json!({ "pairs": &pairs });
        debug!("Querying rates:\n\tURL: {}\n\tBody:{}", url, body);

        let req_start_time = chrono::Utc::now();
        let resp = self
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                Error::HttpRequestError(std::sync::Arc::new(err), "Failed to get rates".to_string())
            })?
            .json::<dto::RatesResponse>()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to parse json while fetching rates".to_string(),
                )
            })?;
        let req_end_time = chrono::Utc::now();

        debug!(
            "rates request took {:?}ms",
            (req_end_time - req_start_time).num_milliseconds()
        );

        Ok(resp)
    }
}

#[derive(Clone, Debug)]
pub struct Rate {
    pub heuristics: HashSet<String>,
    pub rate: Option<BigDecimal>,
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
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(super) struct RatesResponse {
        pub data: Vec<Rate>,
    }

    #[derive(Deserialize)]
    pub(super) struct Rate {
        pub pair: String,
        pub heuristics: Vec<String>,
        pub data: RateData,
    }

    #[derive(Deserialize)]
    pub(super) struct RateData {
        pub rate: Option<BigDecimal>,
        pub heuristic: Option<BigDecimal>,
        pub exchange: Option<BigDecimal>,
    }
}
