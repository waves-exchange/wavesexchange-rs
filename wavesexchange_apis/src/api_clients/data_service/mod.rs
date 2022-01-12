mod impls;

use self::dto::*;
use crate::{ApiBaseUrl, Error};
use async_trait::async_trait;
use chrono::NaiveDateTime;

#[async_trait]
pub trait DataSvcApi: ApiBaseUrl {
    async fn rates<S: Into<String>, I: IntoIterator<Item = (S, S)> + Send, S1: AsRef<str> + Send>(
        &self,
        matcher_address: S1,
        pairs: I,
        timestamp: Option<NaiveDateTime>,
    ) -> Result<Vec<Option<f64>>, Error>;

    // todo proper interface
    async fn invoke_script_transactions(
        &self,
        dapp: impl AsRef<str> + Send + 'async_trait,
        function: impl AsRef<str> + Send + 'async_trait,
        timestamp_lt: impl Into<NaiveDateTime> + Send + 'async_trait,
        // timestamp_gte: NaiveDateTime, todo
        sort: Sort,
        limit: usize,
    ) -> Result<List<InvokeScriptTransaction>, Error>;

    async fn last_exchange_transaction_to_date(
        &self,
        sender: impl AsRef<str> + Send + 'async_trait,
        timestamp: impl Into<NaiveDateTime> + Send + 'async_trait,
    ) -> Result<Option<GenericTransaction>, Error>;
}

pub mod dto {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct List<T> {
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

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Sort {
        Asc,
        Desc,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct RatesRequest {
        pub pairs: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub timestamp: Option<NaiveDateTime>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct RatesResponse {
        pub data: Vec<RateOuter>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct RateOuter {
        pub data: Rate,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct Rate {
        pub rate: f64,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub(super) struct InvokeScriptTransactionResponse {
        pub data: InvokeScriptTransactionData,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct InvokeScriptTransactionData {
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

    #[derive(Debug, Deserialize, Clone)]
    pub(super) struct InvokeScriptCallResponse {
        pub function: String,
        pub args: Vec<InvokeScriptArgumentResponse>,
    }

    #[derive(Debug, Deserialize, Clone)]
    #[serde(tag = "type")]
    pub(super) enum InvokeScriptArgumentResponse {
        #[serde(rename = "string")]
        String { value: String },
        #[serde(rename = "binary")]
        Binary { value: String },
    }

    #[derive(Debug, Clone, Deserialize)]
    pub(super) struct GenericTransactionResponse {
        pub data: GenericTransactionData,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct GenericTransactionData {
        pub id: String,
        pub height: u32,
    }
}

#[derive(Debug, Clone)]
pub enum InvokeScriptArgument {
    String(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct InvokeScriptCall {
    pub function: String,
    pub args: Vec<InvokeScriptArgument>,
}

#[derive(Debug, Clone)]
pub struct InvokeScriptTransaction {
    pub id: String,
    pub height: u32,
    // timestamp: NaiveDateTime, // todo
    pub proofs: Vec<String>,
    pub version: u8,
    //   application_status: TransactionApplicationStatus,
    pub sender: String,
    pub sender_public_key: String,
    pub d_app: String,
    pub call: InvokeScriptCall,
    pub fee: f64,
    // ...
}

#[derive(Debug, Clone)]
pub struct GenericTransaction {
    pub id: String,
    pub height: u32,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{tests::blockchains::MAINNET, HttpClient};
    use chrono::NaiveDate;

    const WAVES: &str = "WAVES";
    const BTC: &str = "8LQW8f7P5d5PZM7GtZEBgaqRPGSzS3DfPuiXrURJ4AJS";
    const NON_TRADABLE_ASSET: &str = "Ej5j5kr1hA4MmdKnewGgG7tJbiHFzotU2x2LELzHjW4o";

    pub fn mainnet_client() -> HttpClient {
        HttpClient::from_base_url(MAINNET::data_service_url)
    }

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
}
