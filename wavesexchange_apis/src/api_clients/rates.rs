use crate::{ApiResult, BaseApi, HttpClient};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct RatesService;

impl BaseApi for RatesService {
    const MAINNET_URL: &'static str = "https://waves.exchange/api/v1/rates";
    const TESTNET_URL: &'static str = "https://testnet.waves.exchange/api/v1/rates";
}

impl HttpClient<RatesService> {
    pub async fn rates(
        &self,
        asset_pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> ApiResult<dto::RatesResponse> {
        let pairs = asset_pairs
            .into_iter()
            .map(|(a, b)| format!("{}/{}", a.into(), b.into()))
            .collect::<Vec<_>>();

        let body = dto::RatesRequest { pairs };

        self.create_req_handler(self.http_post("rates").json(&body), "rates::rates")
            .execute()
            .await
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
