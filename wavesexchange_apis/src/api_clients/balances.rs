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
        let pairs = pairs
            .into_iter()
            .map(|(adress, asset_id)| dto::AddressAssetPair { address, asset_id })
            .collect::<Vec<_>>();

        let mut balances = vec![];

        for chunk_pairs in pairs.chunks(100) {
            let body = dto::BalancesRequest {
                address_asset_pairs: chunk_pairs.to_vec(),
            };

            let mut resp: dto::BalancesResponse = self
                .create_req_handler(
                    self.http_post("balance_history").json(&body),
                    "balances::balance_history",
                )
                .execute()
                .await?;

            balances.append(&mut resp.data);
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
        address_asset_pairs: Vec<AddressAssetPair>,
    }

    #[derive(Debug, Serialize)]
    pub struct AddressAssetPair {
        address: String,
        asset_id: String,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct BalancesResponse {
        pub items: Vec<Balance>,
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct Balance {
        address: String,
        asset_id: String,
        amount: BigDecimal,
        block_height: i32,
        block_timestamp: DateTime,
    }
}
