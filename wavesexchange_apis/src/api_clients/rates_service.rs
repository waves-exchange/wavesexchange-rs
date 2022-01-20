use crate::{BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use itertools::Itertools;
use reqwest::StatusCode;
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
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

        let body = json!({ "pairs": &pairs });

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
        amount_assets: impl IntoIterator<Item = &str> + Send,
        price_asset: impl AsRef<str> + Send,
    ) -> Result<HashMap<String, Rate>, Error> {
        let price_asset = price_asset.as_ref();

        let pairs = amount_assets
            .into_iter()
            .map(|amount_asset| format!("{}/{}", amount_asset, price_asset))
            .collect::<Vec<_>>();

        let body = json!({ "pairs": &pairs });

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
        assets: impl IntoIterator<Item = &str> + Send,
        to_asset: impl AsRef<str> + Send,
    ) -> Result<Option<dto::RatesResponse>, Error> {
        let to_asset = to_asset.as_ref();
        let mut pairs: Vec<_> = vec![];
        pairs.push(format!("{}/{}", "WAVES", to_asset));

        assets
            .into_iter()
            .map(|a| {
                pairs.push(format!("{}/{}", a, to_asset));
            })
            .count();

        let query = json!({ "pairs": &pairs });
        self.create_req_handler(self.post("rates/").json(&query), "rates::exchange_rates")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
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
    pub struct RatesResponse {
        pub data: Vec<Rate>,
    }

    #[derive(Deserialize)]
    pub struct Rate {
        pub pair: String,
        pub heuristics: Vec<String>,
        pub data: RateData,
    }

    #[derive(Deserialize)]
    pub struct RateData {
        pub rate: Option<BigDecimal>,
        pub heuristic: Option<BigDecimal>,
        pub exchange: Option<BigDecimal>,
    }
}
