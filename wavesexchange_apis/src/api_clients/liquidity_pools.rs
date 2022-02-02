use crate::{ApiResult, BaseApi, HttpClient};
use wavesexchange_warp::pagination::List;

#[derive(Clone, Debug)]
pub struct LiquidityPoolsApi;

impl BaseApi for LiquidityPoolsApi {}

impl HttpClient<LiquidityPoolsApi> {
    pub async fn stats(&self) -> ApiResult<List<dto::LiquidityPoolStats>> {
        self.create_req_handler(self.http_get("stats"), "liquidity_pools::stats")
            .execute()
            .await
    }
}

pub mod dto {
    use bigdecimal::BigDecimal;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(tag = "type", rename = "liquidity_pool_stats")]
    pub struct LiquidityPoolStats {
        pub amount_asset_id: String,
        pub price_asset_id: String,
        pub pool_lp_asset_id: String,
        pub volumes: Vec<LiquidityPoolVolume>,
        pub base_apys: Vec<LiquidityPoolBaseApy>,
        pub reward_apy_min: BigDecimal,
        pub reward_apy_max: BigDecimal,
        pub amount_asset_balance: BigDecimal,
        pub price_asset_balance: BigDecimal,
        pub pool_lp_balance: BigDecimal,
        pub current_price: BigDecimal,
        pub lp_amount_asset_share: BigDecimal,
        pub lp_price_asset_share: BigDecimal,
        pub pool_weight: BigDecimal,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct LiquidityPoolVolume {
        pub interval: Interval,
        pub volume: BigDecimal,
        pub quote_volume: BigDecimal,
        pub volume_waves: Option<BigDecimal>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct LiquidityPoolBaseApy {
        pub interval: Interval,
        pub base_apy: BigDecimal,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
    pub enum Interval {
        #[serde(rename = "1d")]
        Day1,
        #[serde(rename = "7d")]
        Days7,
        #[serde(rename = "30d")]
        Days30,
        #[serde(rename = "inf")]
        Infinity,
    }
}
