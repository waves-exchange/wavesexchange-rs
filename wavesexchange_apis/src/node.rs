use super::{Error, HttpClient};
use models::*;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use wavesexchange_log::debug;

#[async_trait]
pub trait NodeApi {
    async fn data_entries(
        &self,
        address: impl AsRef<str> + Send + 'async_trait,
        keys: impl IntoIterator<Item = impl Into<String> + 'async_trait> + Send + 'async_trait,
    ) -> Result<Vec<DataEntry>, Error>;

    async fn evaluate(
        &self,
        dapp: impl AsRef<str> + Send + 'async_trait,
        expression: impl AsRef<str> + Send + 'async_trait,
    ) -> Result<script::Value, Error>;

    async fn get_last_height(&self) -> Result<i32, Error>;
}

#[async_trait]
impl NodeApi for HttpClient {
    async fn data_entries(
        &self,
        address: impl AsRef<str> + Send + 'async_trait,
        keys: impl IntoIterator<Item = impl Into<String>> + Send + 'async_trait,
    ) -> Result<Vec<DataEntry>, Error> {
        let body = StateRequest {
            keys: keys.into_iter().map(Into::into).collect(),
        };

        let endpoint_url = format!("addresses/data/{}", address.as_ref());

        let req_start_time = chrono::Utc::now();

        let resp_raw = self
            .post(&endpoint_url)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to mget data entries from node".to_string(),
                )
            })?;

        let resp_status = resp_raw.status();

        let req_end_time = chrono::Utc::now();
        debug!(
            "node mget data entries request took {:?}ms, status: {:?}",
            (req_end_time - req_start_time).num_milliseconds(),
            resp_status,
        );

        if resp_status == StatusCode::OK {
            let resp: Vec<DataEntryResponse> = resp_raw.json().await.map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to parse json while fetching data entries from node".to_string(),
                )
            })?;

            Ok(resp.into_iter().map(Into::into).collect())
        } else {
            let body = resp_raw.text().await.unwrap_or_else(|_| "".to_owned());
            Err(Error::InvalidStatus(
                    resp_status,
                    format!("Upstream API error while fetching data entries from node. Status {:?}, URL: {}, body: {}", resp_status, endpoint_url, body)
                ))
        }
    }

    async fn evaluate(
        &self,
        dapp: impl AsRef<str> + Send + 'async_trait,
        expression: impl AsRef<str> + Send + 'async_trait,
    ) -> Result<script::Value, Error> {
        let endpoint_url = format!("utils/script/evaluate/{}", dapp.as_ref());
        let body = json!({ "expr": expression.as_ref() });

        let req_start_time = chrono::Utc::now();

        let resp_raw = self
            .post(&endpoint_url)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to the evaluate result from the node".to_string(),
                )
            })?;

        let resp_status = resp_raw.status();

        let req_end_time = chrono::Utc::now();
        debug!(
            "node evaluate request took {:?}ms, status: {}",
            (req_end_time - req_start_time).num_milliseconds(),
            resp_status,
        );

        if resp_status == StatusCode::OK {
            let resp: EvaluateResponse = resp_raw.json().await.map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to parse json while fetching the evaluate result from the node"
                        .to_string(),
                )
            })?;

            Ok(resp.result)
        } else {
            let body = resp_raw.text().await.unwrap_or_else(|_| "".to_owned());
            Err(Error::InvalidStatus(
                    resp_status,
                    format!("Upstream API error while fetching rates from data-service. Status {:?}, URL: {}, body: {}, ", resp_status, endpoint_url, body)
                ))
        }
    }

    async fn get_last_height(&self) -> Result<i32, Error> {
        let r: LastHeight = self
            .get("blocks/height")
            .send()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to get height from node REST api".to_string(),
                )
            })?
            .json()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    std::sync::Arc::new(err),
                    "Failed to parse json while fetching height from node REST api".to_string(),
                )
            })?;

        Ok(r.height)
    }
}

#[derive(Debug, Clone)]
pub struct DataEntry {
    pub key: String,
    pub value: DataEntryValue,
}

#[derive(Debug, Clone)]
pub enum DataEntryValue {
    String(String),
    Integer(i64),
    // Boolean(bool),
    // Binary(Vec<u8>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct LastHeight {
    height: i32,
}

pub mod script {
    use serde::Deserialize;
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
}

mod models {
    use super::{script, DataEntry, DataEntryValue};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize)]
    pub struct StateRequest {
        pub keys: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum DataEntryResponse {
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
    pub struct EvaluateResponse {
        pub result: script::Value,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::config::tests::{MAINNET, TESTNET};

    pub fn mainnet_client() -> HttpClient {
        HttpClient::from_root_url(&MAINNET.upstream.node_url)
    }

    #[tokio::test]
    async fn data_entries() {
        let keys: Vec<String> = ["UAH", "EUR", "CNY", "JPY", "RUB", "NGN"]
            .iter()
            .map(|sym| format!("%s%s__price__{}", sym))
            .collect();

        let data_entries = mainnet_client()
            .data_entries(&MAINNET.addresses.defo_control_contract, keys)
            .await
            .unwrap();

        assert_eq!(data_entries.len(), 6);
        assert_eq!(data_entries.first().unwrap().key, "%s%s__price__UAH");
    }

    use script::*;

    #[tokio::test]
    async fn evaluate() {
        let result = HttpClient::from_root_url(&TESTNET.upstream.node_url)
            .evaluate(
                &TESTNET.any_stake.products[0].contract_address,
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