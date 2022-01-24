use crate::{ApiResult, BaseApi, HttpClient};
use bigdecimal::ToPrimitive;
use wavesexchange_warp::pagination::List;

#[derive(Clone, Debug)]
pub struct LiquidityPoolsApi;

impl BaseApi for LiquidityPoolsApi {}

impl HttpClient<LiquidityPoolsApi> {
    pub async fn stats(&self, asset_id: Option<&str>) -> ApiResult<Vec<LiquidityPoolBrief>> {
        let response: List<dto::LiquidityPoolStats> = self
            .create_req_handler(self.get("stats"), "liquidity_pools::stats")
            .execute()
            .await?;
        let pools = response.items.iter();
        Ok(match asset_id {
            Some(asset) => pools
                .filter_map(|lps| {
                    if &lps.pool_lp_asset_id == asset {
                        Some(lps.into())
                    } else {
                        None
                    }
                })
                .collect(),
            None => pools.map(LiquidityPoolBrief::from).collect(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct LiquidityPoolBrief {
    pub pool_lp_asset_id: String,
    pub reward_apy_min: f64,
    pub reward_apy_max: f64,
    pub base_apy_1d: f64,
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

impl From<&dto::LiquidityPoolStats> for LiquidityPoolBrief {
    fn from(lps: &dto::LiquidityPoolStats) -> Self {
        LiquidityPoolBrief {
            pool_lp_asset_id: lps.pool_lp_asset_id.clone(),
            reward_apy_min: lps.reward_apy_min.to_f64().unwrap_or_default(),
            reward_apy_max: lps.reward_apy_max.to_f64().unwrap_or_default(),
            base_apy_1d: lps
                .base_apys
                .iter()
                .find(|apy| apy.interval == dto::Interval::Day1)
                .unwrap()
                .base_apy
                .to_f64()
                .unwrap_or_default(),
        }
    }
}
