use self::dto::*;
use crate::models::{DataEntry, DataEntryValue};
use crate::{BaseApi, Error, HttpClient};
use reqwest::StatusCode;
use serde_json::json;
use wavesexchange_log::debug;

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

    pub async fn evaluate(
        &self,
        dapp: impl AsRef<str> + Send,
        expression: impl AsRef<str> + Send,
    ) -> Result<Value, Error> {
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

    pub async fn get_last_height(&self) -> Result<i32, Error> {
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

pub mod dto {
    use super::*;
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
}

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;

    pub fn mainnet_client() -> HttpClient<NodeApi> {
        HttpClient::from_base_url(MAINNET::state_service_url)
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
