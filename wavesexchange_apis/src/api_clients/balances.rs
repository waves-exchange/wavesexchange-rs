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
            None => "balance_history".into()
        };

        let pairs = pairs
            .into_iter()
            .map(|(address, asset_id)| dto::AddressAssetPair {
                address: address.into(),
                asset_id: asset_id.into(),
            })
            .collect::<Vec<_>>();

        let mut balances = vec![];

        for chunk_pairs in pairs.chunks(100) {
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
    pub struct Balance {
        pub address: String,
        pub asset_id: String,
        pub amount: BigDecimal,
        pub block_height: i32,
        pub block_timestamp: DateTime<Utc>,
    }
}
