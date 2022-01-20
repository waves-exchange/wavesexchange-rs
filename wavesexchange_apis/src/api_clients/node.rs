use self::dto::*;
use crate::models::DataEntryValue;
use crate::{BaseApi, Error, HttpClient};
use itertools::join;
use reqwest::StatusCode;
use serde_json::json;

#[derive(Clone)]
pub struct NodeApi;

impl BaseApi for NodeApi {}

impl HttpClient<NodeApi> {
    pub async fn data_entries(
        &self,
        address: impl AsRef<str> + Send,
        keys: impl IntoIterator<Item = impl Into<String>> + Send,
    ) -> Result<Vec<DataEntry>, Error> {
        let body = StateRequest {
            keys: keys.into_iter().map(Into::into).collect(),
        };
        let endpoint_url = format!("addresses/data/{}", address.as_ref());

        let resp: Vec<DataEntryResponse> = self
            .create_req_handler(self.post(&endpoint_url).json(&body), "node::data_entries")
            .execute()
            .await?;

        Ok(resp.into_iter().map(Into::into).collect())
    }

    pub async fn evaluate(
        &self,
        dapp: impl AsRef<str> + Send,
        expression: impl AsRef<str> + Send,
    ) -> Result<Value, Error> {
        let endpoint_url = format!("utils/script/evaluate/{}", dapp.as_ref());
        let body = json!({ "expr": expression.as_ref() });

        let resp: EvaluateResponse = self
            .create_req_handler(self.post(&endpoint_url).json(&body), "node::evaluate")
            .execute()
            .await?;

        Ok(resp.result)
    }

    pub async fn get_last_height(&self) -> Result<i32, Error> {
        let r: LastHeight = self
            .create_req_handler(self.get("blocks/height"), "node::get_last_height")
            .execute()
            .await?;

        Ok(r.height)
    }

    pub async fn matcher_waves_balance(
        &self,
        address: &String,
    ) -> Result<Option<MatcherWavesBalance>, Error> {
        let url = format!("addresses/balance/details/{address}");
        self.create_req_handler(self.get(url), "node::matcher_waves_balance")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn balances_on_matcher(
        &self,
        address: &String,
        asset_ids: Vec<&String>,
    ) -> Result<Option<MatcherBalances>, Error> {
        let url = format!("assets/balance/{address}");
        let data = json!({ "ids": asset_ids });
        self.create_req_handler(self.post(url).json(&data), "node::balances_on_matcher")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn assets_details(
        &self,
        assets: &Vec<&String>,
    ) -> Result<Option<Vec<AssetDetail>>, Error> {
        let url = format!(
            "assets/details?id={}",
            join(
                assets.iter().filter(|e| { !e.as_str().eq("WAVES") }),
                "&id="
            )
        );

        self.create_req_handler(self.get(url), "node::assets_details")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }
}

pub mod dto {
    use super::*;
    use bigdecimal::BigDecimal;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Clone, Deserialize)]
    pub struct IntValue {
        pub value: i64,
    }
    #[derive(Debug, Clone, Deserialize)]
    pub struct StringValue {
        pub value: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct IntegerEntryValue {
        pub key: StringValue,
        pub value: IntValue,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(tag = "type")]
    pub enum Value {
        Array { value: Vec<Value> },
        Tuple { value: HashMap<String, Value> },
        IntegerEntry { value: IntegerEntryValue },
        String { value: String },
        Int { value: i64 },
        // todo other types
    }

    #[derive(Debug, Clone, Deserialize)]
    pub(super) struct LastHeight {
        pub height: i32,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct StateRequest {
        pub keys: Vec<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct DataEntry {
        pub key: String,
        pub value: DataEntryValue,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub(super) enum DataEntryResponse {
        #[serde(rename = "string")]
        String { key: String, value: String },
        #[serde(rename = "integer")]
        Integer { key: String, value: i64 },
    }

    impl From<DataEntryResponse> for DataEntry {
        fn from(de: DataEntryResponse) -> Self {
            match de {
                DataEntryResponse::String { key: k, value: v } => DataEntry {
                    key: k,
                    value: DataEntryValue::String(v),
                },
                DataEntryResponse::Integer { key: k, value: v } => DataEntry {
                    key: k,
                    value: DataEntryValue::Integer(v),
                },
            }
        }
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct EvaluateResponse {
        pub result: Value,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MatcherWavesBalance {
        available: BigDecimal,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MatcherBalances {
        pub address: String,
        pub balances: Vec<BalanceItem>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BalanceItem {
        #[serde(rename(deserialize = "assetId"))]
        pub asset_id: String,
        pub balance: u64,
        pub quantity: Option<u64>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AssetDetailItem {
        #[serde(rename(deserialize = "assetId"))]
        pub asset_id: String,
        decimals: u8,
        description: String,
        name: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AssetDetailError {
        pub error: isize,
        pub message: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum AssetDetail {
        Ok(AssetDetailItem),
        Err(AssetDetailError),
    }
}

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;

    pub fn mainnet_client() -> HttpClient<NodeApi> {
        HttpClient::from_base_url(MAINNET::node_url)
    }
}

#[cfg(test)]
mod tests_internal {
    use super::tests::*;
    use super::*;
    use crate::tests::blockchains::{MAINNET, TESTNET};

    #[tokio::test]
    async fn data_entries() {
        let keys: Vec<String> = ["UAH", "EUR", "CNY", "JPY", "RUB", "NGN"]
            .iter()
            .map(|sym| format!("%s%s__price__{}", sym))
            .collect();

        let data_entries = mainnet_client()
            .data_entries(MAINNET::defo_control_contract, keys)
            .await
            .unwrap();

        assert_eq!(data_entries.len(), 6);
        assert_eq!(data_entries.first().unwrap().key, "%s%s__price__UAH");
    }

    #[tokio::test]
    async fn evaluate() {
        let result = HttpClient::<NodeApi>::from_base_url(TESTNET::node_url)
            .evaluate(
                &TESTNET::products[0].contract_address,
                "privateCurrentSysParamsREST(\"5Sh9KghfkZyhjwuodovDhB6PghDUGBHiAPZ4MkrPgKtX\")",
            )
            .await
            .unwrap();

        match result {
            Value::Tuple { value } => {
                let price = match value.get("_1") {
                    Some(Value::IntegerEntry {
                        value:
                            IntegerEntryValue {
                                value: IntValue { value },
                                ..
                            },
                    }) => value.to_owned(),
                    _ => panic!(),
                };

                let decimals_mult = match value.get("_2") {
                    Some(Value::IntegerEntry {
                        value:
                            IntegerEntryValue {
                                value: IntValue { value },
                                ..
                            },
                    }) => value.to_owned(),
                    _ => panic!(),
                };

                assert!(price > 0);
                assert!(decimals_mult > 0);
            }
            _ => panic!(),
        };
    }
}
