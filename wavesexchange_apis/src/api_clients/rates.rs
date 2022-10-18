use itertools::Itertools;

use crate::{ApiResult, BaseApi, HttpClient};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct RatesService;

impl BaseApi for RatesService {}

impl HttpClient<RatesService> {
    pub async fn rates(
        &self,
        asset_pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> ApiResult<dto::RatesResponse> {
        let pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| format!("{}/{}", a.into(), b.into()))
            .collect::<Vec<_>>();

        let mut rates = vec![];

        for w_pairs in &pairs.into_iter().chunks(100) {
            chunk_pairs = w_pairs.collect();

            dbg!(&chunk_pairs);

            let body = dto::RatesRequest { pairs: chunk_pairs };
            let mut req: dto::RatesResponse = self
                .create_req_handler(self.http_post("rates").json(&body), "rates::rates")
                .execute()
                .await?;

            rates.append(&mut req.data);
        }

        Ok(dto::RatesResponse { data: rates })
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
