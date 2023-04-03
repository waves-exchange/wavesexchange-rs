use crate::{ApiResult, BaseApi, HttpClient};
use reqwest::StatusCode;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct Node;

impl BaseApi for Node {}

impl HttpClient<Node> {
    pub async fn data_entries(
        &self,
        address: impl AsRef<str>,
        keys: impl IntoIterator<Item = impl Into<String>>,
    ) -> ApiResult<Vec<dto::DataEntryResponse>> {
        let body = dto::StateRequest {
            keys: keys.into_iter().map(Into::into).collect(),
        };
        let endpoint_url = format!("addresses/data/{}", address.as_ref());

        self.create_req_handler(
            self.http_post(&endpoint_url).json(&body),
            "node::data_entries",
        )
        .execute()
        .await
    }

    pub async fn evaluate(
        &self,
        dapp: impl AsRef<str>,
        expression: impl AsRef<str>,
    ) -> ApiResult<dto::EvaluateResponse> {
        let endpoint_url = format!("utils/script/evaluate/{}", dapp.as_ref());
        let body = json!({ "expr": expression.as_ref() });

        self.create_req_handler(self.http_post(&endpoint_url).json(&body), "node::evaluate")
            .execute()
            .await
    }

    pub async fn get_last_height(&self) -> ApiResult<dto::LastHeight> {
        self.create_req_handler(self.http_get("blocks/height"), "node::get_last_height")
            .execute()
            .await
    }

    pub async fn addr_balance_details(
        &self,
        address: impl AsRef<str>,
    ) -> ApiResult<Option<dto::MatcherWavesBalance>> {
        let url = format!("addresses/balance/details/{}", address.as_ref());
        self.create_req_handler(self.http_get(url), "node::addr_balance_details")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn assets_balance(
        &self,
        address: impl AsRef<str>,
        asset_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> ApiResult<Option<dto::MatcherBalances>> {
        let url = format!("assets/balance/{}", address.as_ref());
        let asset_ids = asset_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        let data = json!({ "ids": asset_ids });
        self.create_req_handler(self.http_post(url).json(&data), "node::assets_balance")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn assets_details(
        &self,
        assets: impl IntoIterator<Item = impl Into<String>>,
    ) -> ApiResult<Option<Vec<dto::AssetDetail>>> {
        let url = "assets/details".to_string();
        let asset_ids = assets.into_iter().map(Into::into).collect::<Vec<_>>();
        let data = json!({ "ids": asset_ids });
        self.create_req_handler(self.http_post(url).json(&data), "node::assets_details")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn transaction_broadcast(&self, transaction: String) -> ApiResult<serde_json::Value> {
        self.create_req_handler(
            self.http_post("transactions/broadcast")
                .header("Content-Type", "application/json")
                .body(transaction.into_bytes()),
            "node::transaction_broadcast",
        )
        .execute()
        .await
    }

    pub async fn state_changes_by_address(
        &self,
        address: impl AsRef<str>,
        limit: usize,
        cursor: Option<String>,
    ) -> ApiResult<Vec<dto::StateChangesResponse>> {
        let url = format!(
            "debug/stateChanges/address/{}/limit/{limit}{query_string}",
            address.as_ref(),
            query_string = match &cursor {
                None => String::from(""),
                Some(id) => format!("?after={}", id),
            }
        );

        self.create_req_handler(self.http_get(url), "node::state_changes_by_address")
            .execute()
            .await
    }

    pub async fn state_changes_by_transaction_id(
        &self,
        transaction_id: impl AsRef<str>,
    ) -> ApiResult<dto::StateChangesResponse> {
        let url = format!("debug/stateChanges/info/{}", transaction_id.as_ref());
        self.create_req_handler(self.http_get(url), "node::state_changes_by_transaction_id")
            .execute()
            .await
    }
}

pub mod dto {
    use crate::models::dto::{DataEntryValue, TypeError};
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

    #[derive(Clone, Debug, thiserror::Error)]
    #[error("Expected tuple of {0} elements, missing key '_{1}'")]
    pub struct TupleError(u8, u8);

    #[derive(Debug, Clone, Deserialize)]
    pub struct LastHeight {
        pub height: i32,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct StateRequest {
        pub keys: Vec<String>,
    }

    //TODO Most likely this `DataEntryResponse` needs to be merged with `models::dto::DataEntryValue`
    // or at least be convertable to it.
    // Need to make a convenient API here - separate key (which is just repeated in every branch of the enum) from value etc.

    #[derive(Debug, Deserialize, Clone)]
    #[serde(tag = "type")]
    pub enum DataEntryResponse {
        #[serde(rename = "string")]
        String { key: String, value: String },
        #[serde(rename = "integer")]
        Integer { key: String, value: i64 },
        #[serde(rename = "boolean")]
        Boolean { key: String, value: bool },
        #[serde(rename = "binary")]
        Binary { key: String, value: Vec<u8> },
    }

    #[derive(Debug, Deserialize)]
    pub struct EvaluateResponse {
        pub result: Value,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MatcherWavesBalance {
        pub available: BigDecimal,
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
        pub decimals: u8,
        pub description: String,
        pub name: String,
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

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateChangesResponse {
        #[serde(rename = "id")]
        pub transaction_id: String,
        pub height: i32,
        pub timestamp: u64,
        pub sender: String,
        #[serde(rename = "type")]
        pub transaction_type: u8,
        #[serde(rename = "stateChanges")]
        pub state_changes: Option<StateChangesResponseDataList>,
        #[serde(rename = "dApp")]
        pub dapp: Option<String>,
        pub call: Option<StateChangesResponseCall>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateChangesResponseCall {
        pub function: String,
        pub args: Vec<ArgumentResponse>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateChangesResponseDataList {
        pub data: Vec<DataEntryResponse>,
        pub transfers: Vec<TransferResponse>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct TransferResponse {
        pub address: String,
        pub asset: Option<String>,
        pub amount: i64,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(tag = "type")]
    pub enum ArgumentResponse {
        #[serde(rename = "integer")]
        Integer { value: i64 },
        #[serde(rename = "string")]
        String { value: String },
        // todo rest of them
    }

    impl DataEntryResponse {
        #[inline]
        pub fn key(&self) -> &str {
            match self {
                DataEntryResponse::String { key, .. }
                | DataEntryResponse::Integer { key, .. }
                | DataEntryResponse::Boolean { key, .. }
                | DataEntryResponse::Binary { key, .. } => key.as_str(),
            }
        }

        #[inline]
        pub fn into_value(self) -> DataEntryValue {
            match self {
                DataEntryResponse::String { value, .. } => DataEntryValue::String(value),
                DataEntryResponse::Integer { value, .. } => DataEntryValue::Integer(value),
                DataEntryResponse::Boolean { value, .. } => DataEntryValue::Boolean(value),
                DataEntryResponse::Binary { value, .. } => DataEntryValue::Binary(value),
            }
        }

        #[inline]
        pub fn into_key_value(self) -> (String, DataEntryValue) {
            match self {
                DataEntryResponse::String { key, value } => (key, DataEntryValue::String(value)),
                DataEntryResponse::Integer { key, value } => (key, DataEntryValue::Integer(value)),
                DataEntryResponse::Boolean { key, value } => (key, DataEntryValue::Boolean(value)),
                DataEntryResponse::Binary { key, value } => (key, DataEntryValue::Binary(value)),
            }
        }
    }

    impl Value {
        #[inline]
        pub fn try_as_array(&self) -> Result<&[Value], TypeError> {
            match self {
                Value::Array { value } => Ok(value.as_slice()),
                _ => Err(self.type_error("Array")),
            }
        }

        #[inline]
        pub fn try_into_array(self) -> Result<Vec<Value>, TypeError> {
            match self {
                Value::Array { value } => Ok(value),
                _ => Err(self.type_error("Array")),
            }
        }

        #[inline]
        pub fn try_into_tuple_2(self) -> Result<Result<(Value, Value), TupleError>, TypeError> {
            match self {
                Value::Tuple { value } => Ok(Self::hash_map_into_tuple_2(value)),
                _ => Err(self.type_error("Tuple")),
            }
        }

        fn hash_map_into_tuple_2(
            mut map: HashMap<String, Value>,
        ) -> Result<(Value, Value), TupleError> {
            let v1 = map.remove("_1").ok_or_else(|| TupleError(2, 1))?;
            let v2 = map.remove("_2").ok_or_else(|| TupleError(2, 1))?;
            Ok((v1, v2))
        }

        #[inline]
        pub fn try_as_str(&self) -> Result<&str, TypeError> {
            match self {
                Value::String { value } => Ok(value.as_str()),
                _ => Err(self.type_error("String")),
            }
        }

        #[inline]
        pub fn try_into_string(self) -> Result<String, TypeError> {
            match self {
                Value::String { value } => Ok(value),
                _ => Err(self.type_error("String")),
            }
        }

        #[inline]
        pub fn try_as_int(&self) -> Result<i64, TypeError> {
            match self {
                Value::Int { value } => Ok(*value),
                _ => Err(self.type_error("Int")),
            }
        }

        #[inline]
        pub fn try_into_int(self) -> Result<i64, TypeError> {
            match self {
                Value::Int { value } => Ok(value),
                _ => Err(self.type_error("Int")),
            }
        }

        #[inline]
        pub fn value_type_name(&self) -> &'static str {
            match self {
                Value::Array { .. } => "Array",
                Value::Tuple { .. } => "Tuple",
                Value::IntegerEntry { .. } => "IntegerEntry",
                Value::String { .. } => "String",
                Value::Int { .. } => "Int",
            }
        }

        #[inline]
        fn type_error(&self, expected: &'static str) -> TypeError {
            TypeError(self.value_type_name(), expected)
        }
    }
}
