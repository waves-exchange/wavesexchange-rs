use super::{
    dto, DataSvcApi, GenericTransaction, InvokeScriptArgument, InvokeScriptCall,
    InvokeScriptTransaction, List, Sort,
};
use crate::{Error, HttpClient};
use chrono::NaiveDateTime;
use wavesexchange_log::debug;

const HEADER_ORIGIN_NAME: &str = "Origin";
const HEADER_ORIGIN_VALUE: &str = "waves.exchange";

impl HttpClient<DataSvcApi> {
    pub async fn rates<
        S: Into<String>,
        I: IntoIterator<Item = (S, S)> + Send,
        S1: AsRef<str> + Send,
    >(
        &self,
        matcher_address: S1,
        pairs: I,
        timestamp: Option<NaiveDateTime>,
    ) -> Result<Vec<Option<f64>>, Error> {
        let req = dto::RatesRequest {
            pairs: pairs
                .into_iter()
                .map(|(amt, pr)| amt.into() + "/" + &pr.into())
                .collect(),
            timestamp,
        };

        let url = format!("matchers/{}/rates", matcher_address.as_ref());

        let resp: dto::RatesResponse = self
            .create_req_handler(
                self.post(&url)
                    .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE)
                    .json(&req),
                "data_service::rates",
            )
            .execute()
            .await?;

        Ok(resp
            .data
            .into_iter()
            .map(|r| {
                if r.data.rate == 0.0 {
                    None
                } else {
                    Some(r.data.rate)
                }
            })
            .collect())
    }

    pub async fn invoke_script_transactions(
        &self,
        dapp: impl AsRef<str> + Send,
        function: impl AsRef<str> + Send,
        timestamp_lt: impl Into<NaiveDateTime> + Send,
        // timestamp_gte: NaiveDateTime,
        sort: Sort,
        limit: usize,
    ) -> Result<List<InvokeScriptTransaction>, Error> {
        let url = format!(
            "transactions/invoke-script?dapp={}&function={}&timeEnd={:?}&sort={}&limit={}",
            dapp.as_ref(),
            function.as_ref(),
            timestamp_lt.into(),
            sort,
            limit,
        );

        let resp: List<dto::InvokeScriptTransactionResponse> = self
            .create_req_handler(
                self.get(&url)
                    .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE),
                "data_service::invoke_script_transactions",
            )
            .execute()
            .await?;

        let list: List<InvokeScriptTransaction> = List {
            data: resp.data.into_iter().map(Into::into).collect(),
            last_cursor: resp.last_cursor,
            is_last_page: resp.is_last_page,
        };

        Ok(list)
    }

    pub async fn last_exchange_transaction_to_date(
        &self,
        sender: impl AsRef<str> + Send,
        timestamp: impl Into<NaiveDateTime> + Send,
    ) -> Result<Option<GenericTransaction>, Error> {
        let url = format!(
            "transactions/exchange?sender={}&timeEnd={:?}&limit=1",
            sender.as_ref(),
            timestamp.into(),
        );

        let resp: List<dto::GenericTransactionResponse> = self
            .create_req_handler(
                self.get(&url)
                    .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE),
                "data_service::last_exchange_transaction_to_date",
            )
            .execute()
            .await?;

        if resp.data.is_empty() {
            debug!(
                "Data service: no transactions found for sender={}",
                sender.as_ref()
            );
            return Ok(None);
        }

        let list: List<GenericTransaction> = List {
            data: resp.data.into_iter().map(Into::into).collect(),
            last_cursor: resp.last_cursor,
            is_last_page: resp.is_last_page,
        };

        if list.data.len() == 1 {
            let trans = list.data.into_iter().next().unwrap();
            Ok(Some(trans))
        } else {
            Err(Error::ResponseParseError(format!(
                "Failed to interpret data, expected one transaction, found {}",
                list.data.len()
            )))
        }
    }
}

// conversions
impl From<dto::InvokeScriptCallResponse> for InvokeScriptCall {
    fn from(value: dto::InvokeScriptCallResponse) -> Self {
        Self {
            function: value.function,
            args: value.args.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<dto::InvokeScriptTransactionResponse> for InvokeScriptTransaction {
    fn from(value: dto::InvokeScriptTransactionResponse) -> Self {
        Self {
            id: value.data.id,
            height: value.data.height,
            proofs: value.data.proofs,
            version: value.data.version,
            sender: value.data.sender,
            sender_public_key: value.data.sender_public_key,
            d_app: value.data.d_app,
            call: value.data.call.into(),
            fee: value.data.fee,
        }
    }
}

impl From<dto::InvokeScriptArgumentResponse> for InvokeScriptArgument {
    fn from(value: dto::InvokeScriptArgumentResponse) -> Self {
        match value {
            dto::InvokeScriptArgumentResponse::String { value: v } => {
                InvokeScriptArgument::String(v)
            }
            // data validated by the blockchain should be pretty trustworthy, hence .unwrap()
            dto::InvokeScriptArgumentResponse::Binary { value: v } => {
                InvokeScriptArgument::Binary(base64::decode(&v[7..]).unwrap())
            }
        }
    }
}

impl From<dto::GenericTransactionResponse> for GenericTransaction {
    fn from(value: dto::GenericTransactionResponse) -> Self {
        Self {
            id: value.data.id,
            height: value.data.height,
        }
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
