mod impls;

use self::dto::*;
use crate::BaseApi;

#[derive(Clone, Debug)]
pub struct DataService;

impl BaseApi for DataService {
    const MAINNET_URL: &'static str = "https://api.wavesplatform.com/v0";
    const TESTNET_URL: &'static str = "https://api-testnet.wavesplatform.com/v0";
}

pub mod dto {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DSList<T> {
        pub data: Vec<T>,
        pub last_cursor: Option<String>,
        pub is_last_page: bool,
    }

    #[derive(Debug, Deserialize, Clone)]
    #[serde(rename_all = "snake_case")]
    pub enum TransactionApplicationStatus {
        Succeeded,
        Failed,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Sort {
        Asc,
        Desc,
    }

    #[derive(Debug, Clone, Deserialize)]

    pub struct AssetInfo {
        pub id: String,
        pub precision: u8,
        pub ticker: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ExchangeTransaction {
        // only trade-related data so far
        pub id: String,
        pub timestamp: DateTime<Utc>,
        pub amount: f64,
        pub price: f64,
        pub order1: Order,
        pub order2: Order,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[allow(non_snake_case)]
    pub struct InvokeScriptTransactionRequest {
        pub sender: Option<String>,
        pub senders: Option<Vec<String>>,
        pub timeStart: Option<NaiveDateTime>,
        pub timeEnd: Option<NaiveDateTime>,
        pub dapp: Option<String>,
        pub function: Option<String>,
        pub after: Option<String>,
        pub sort: Option<Sort>,
        pub limit: Option<usize>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Order {
        // only trade-related data so far
        pub sender: String,
        pub order_type: OrderType,
        pub asset_pair: AssetPair,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AssetPair {
        pub amount_asset: String,
        pub price_asset: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum OrderType {
        Buy,
        Sell,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Data<T> {
        pub data: T,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct ExchangeTransactionsQueryParams {
        pub amount_asset: Option<String>,
        pub price_asset: Option<String>,
        pub sender: Option<String>,
        pub time_start: Option<DateTime<Utc>>,
        pub time_end: Option<DateTime<Utc>>,
        pub sort: Sort,
        pub limit: usize,
        pub after: Option<String>,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct RatesRequest {
        pub pairs: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub timestamp: Option<NaiveDateTime>,
    }

    #[derive(Debug, Deserialize)]
    pub struct RatesResponse {
        pub data: Vec<RateOuter>,
    }

    #[derive(Debug, Deserialize)]
    pub struct RateOuter {
        pub data: Rate,
    }

    #[derive(Debug, Deserialize)]
    pub struct Rate {
        pub rate: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InvokeScriptTransactionResponse {
        pub data: InvokeScriptTransactionData,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InvokeScriptTransactionData {
        pub id: String,
        pub height: u32,
        // timestamp: NaiveDateTime, // todo
        pub proofs: Vec<String>,
        pub version: u8,
        // application_status: TransactionApplicationStatus,
        pub sender: String,
        pub sender_public_key: String,
        pub d_app: String,
        pub call: InvokeScriptCallResponse,
        pub fee: f64,
        // ...
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct InvokeScriptCallResponse {
        pub function: String,
        pub args: Vec<InvokeScriptArgumentResponse>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    #[serde(tag = "type")]
    pub enum InvokeScriptArgumentResponse {
        #[serde(rename = "string")]
        String { value: String },
        #[serde(rename = "binary")]
        Binary { value: String },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GenericTransactionResponse {
        pub data: GenericTransactionData,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GenericTransactionData {
        pub id: String,
        pub height: u32,
    }
}

impl core::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Sort::Asc => write!(f, "asc"),
            Sort::Desc => write!(f, "desc"),
        }
    }
}
