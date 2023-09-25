use chrono::{DateTime, Utc};

use crate::{ApiResult, BaseApi, HttpClient};
use std::fmt::Debug;

const CHUNK_SIZE: usize = 100;

#[derive(Clone, Debug)]
pub struct BalancesService;

pub enum BlockRef {
    Height(i32),
    Timestamp(DateTime<Utc>)
}

impl BaseApi for BalancesService {}

impl HttpClient<BalancesService> {
    pub async fn balance_history(
        &self,
        pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        block_ref: Option<BlockRef>,
    ) -> ApiResult<dto::BalancesResponse> {
        let balances_url = match block_ref {
            Some(BlockRef::Height(h)) => format!("balance_history?height={}", h),
            Some(BlockRef::Timestamp(t)) => format!("balance_history?timestamp={}", t.format("%Y-%m-%dT%H:%M:%SZ")),
            None => "balance_history".into(),
        };

        let pairs = pairs
            .into_iter()
            .map(|(address, asset_id)| dto::AddressAssetPair {
                address: address.into(),
                asset_id: asset_id.into(),
            })
            .collect::<Vec<_>>();

        let mut balances = vec![];

        for chunk_pairs in pairs.chunks(CHUNK_SIZE) {
            let body = dto::BalancesRequest {
                address_asset_pairs: chunk_pairs.to_vec(),
            };

            let mut resp: dto::BalancesResponse = self
                .create_req_handler(
                    self.http_post(balances_url.clone()).json(&body),
                    "balances::balance_history",
                )
                .execute()
                .await?;

            balances.append(&mut resp.items);
        }
        Ok(dto::BalancesResponse { items: balances })
    }

    pub async fn balance_aggregates(
        &self,
        address: impl Into<String>,
        asset_id: impl Into<String>,
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> ApiResult<dto::BalancesAggResponse> {
        let mut url = format!(
            "balance_history/aggregates/{}/{}",
            address.into(),
            asset_id.into()
        );

        match (date_from, date_to) {
            (Some(f), Some(d)) => url = format!("{}?date_from={}&date_to={}", url, f, d),
            (Some(f), None) => url = format!("{}?date_from={}", url, f),
            (None, Some(d)) => url = format!("{}?date_to={}", url, d),
            (None, None) => {}
        }

        let resp: dto::BalancesAggResponse = self
            .create_req_handler(
                self.http_get(url.clone()),
                "balances::balance_history/aggregates",
            )
            .execute()
            .await?;

        Ok(dto::BalancesAggResponse { items: resp.items })
    }
}

pub mod dto {
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize)]
    pub struct BalancesRequest {
        pub address_asset_pairs: Vec<AddressAssetPair>,
    }

    #[derive(Debug, Serialize, Clone)]
    pub struct AddressAssetPair {
        pub address: String,
        pub asset_id: String,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct BalancesResponse {
        pub items: Vec<Balance>,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct BalancesAggResponse {
        pub items: Vec<BalanceAggItem>,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct Balance {
        pub address: String,
        pub asset_id: String,
        pub amount: BigDecimal,
        pub block_height: i32,
        pub block_timestamp: DateTime<Utc>,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct BalanceAggItem {
        pub amount_begin: BigDecimal,
        pub amount_end: BigDecimal,
        pub date_stamp: DateTime<Utc>,
    }
}
