use chrono::{DateTime, Utc};

use crate::{ApiResult, BaseApi, HttpClient};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct BalancesService;

impl BaseApi for BalancesService {}

impl HttpClient<BalancesService> {
    pub async fn balance_history(
        &self,
        pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        height: Option<i32>,
    ) -> ApiResult<dto::BalancesResponse> {
        let balances_url = match height {
            Some(h) => format!("balance_history?height={}", h),
            None => "balance_history".into(),
        };

        let mut balances = vec![];

        pairs
            .into_iter()
            .map(|(address, asset_id)| dto::AddressAssetPair {
                address: address.into(),
                asset_id: asset_id.into(),
            })
            .chunks(100)
            .for_each(|chunk| {
                let body = dto::BalancesRequest {
                    address_asset_pairs: chunk.to_vec(),
                };

                let mut resp: dto::BalancesResponse = self
                    .create_req_handler(
                        self.http_post(balances_url.clone()).json(&body),
                        "balances::balance_history",
                    )
                    .execute()
                    .await?;

                balances.append(&mut resp.items);
            });

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

        let mut resp: dto::BalancesAggResponse = self
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
