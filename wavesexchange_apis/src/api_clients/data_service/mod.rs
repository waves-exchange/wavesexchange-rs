mod impls;

use self::dto::*;
use crate::{BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct DataSvcApi;

impl BaseApi for DataSvcApi {}

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

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;

    pub fn mainnet_client() -> HttpClient<DataSvcApi> {
        HttpClient::from_base_url(MAINNET::data_service_url)
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

#[cfg(test)]
mod tests_internal {
    use super::tests::*;
    use super::*;
    use crate::tests::blockchains::MAINNET;
    use chrono::{Date, NaiveDate, Utc};

    const WAVES: &str = "WAVES";
    const BTC: &str = "8LQW8f7P5d5PZM7GtZEBgaqRPGSzS3DfPuiXrURJ4AJS";
    const NON_TRADABLE_ASSET: &str = "Ej5j5kr1hA4MmdKnewGgG7tJbiHFzotU2x2LELzHjW4o";
    const USDN_ASSET_ID: &str = "DG2xFkPdDwKUoBkzGAhQtLpSGzfXLiCYPEzeKH2Ad24p";

    #[tokio::test]
    async fn fetch_rates_batch_from_data_service() {
        let rates = mainnet_client()
            .rates(
                MAINNET::matcher,
                vec![(WAVES, BTC), (NON_TRADABLE_ASSET, WAVES)],
                None,
            )
            .await
            .unwrap();

        assert_eq!(rates.len(), 2);
        assert!(rates[0].unwrap() > 0.0);
        assert!(rates[1].is_none());
    }

    #[tokio::test]
    async fn fetch_invokes_control_contract_finalize_current_price_v2() {
        // example invoke TS: 2021-06-21T16:38:52
        let timestamp_lt = NaiveDate::from_ymd(2021, 06, 21).and_hms(16, 38, 53);

        let invokes = mainnet_client()
            .invoke_script_transactions(
                MAINNET::defo_control_contract,
                "finalizeCurrentPriceV2",
                timestamp_lt,
                Sort::Desc,
                3,
            )
            .await
            .unwrap();

        assert_eq!(invokes.data.len(), 3);

        let tx = invokes.data[0].clone();
        assert_eq!(tx.id, "2i6b9EksSrVS4dpX7LNxDisuR4JAgoNaCQSmXK1Gjwia");
        assert_eq!(tx.height, 2645143);
        assert_eq!(tx.proofs, ["4Msv4pGbR8wnmdHtzoWe6cr6eMRnmzBfNANX7jEpVDxKme8cQqtNzu9CYc4JUvhh52DmPpiuWCpe1DHN2ZGCruoG"]);
        assert_eq!(tx.version, 1);
        assert_eq!(tx.sender, "3PHYJhQ7WzGqvrPrbE5YKutSGHo4K2JRg42");
        assert_eq!(
            tx.sender_public_key,
            "8TLsCqkkroVot9dVR1WcWUN9Qx96HDfzG3hnx7NpSJA9"
        );
        assert_eq!(tx.fee, 0.005);

        // args with updated prices
        match &tx.call.args[1] {
            InvokeScriptArgument::Binary(_) => panic!(),
            InvokeScriptArgument::String(updated_prices) => assert_eq!(
                updated_prices,
                "BRL_5034198_1_UAH_27269793_1_GBP_718250_1_TRY_8764295_1"
            ),
        }
    }

    #[tokio::test]
    async fn get_exchange_transactions() {
        let date = Date::from_utc(NaiveDate::from_ymd(2021, 05, 01), Utc);

        let txs_resp = mainnet_client()
            .transactions_exchange(
                Option::<String>::None,
                Some(WAVES),
                Some(USDN_ASSET_ID),
                Some(date.and_hms(0, 0, 0)),
                Some(date.and_hms(0, 30, 0)),
                Sort::Desc,
                3,
                Option::<String>::None,
            )
            .await
            .unwrap();

        assert_eq!(txs_resp.items.len(), 3);
    }
}
