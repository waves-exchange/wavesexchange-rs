use super::{dto, DSList, DataService, InvokeScriptTransactionRequest, Sort};
use crate::{ApiResult, HttpClient};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wavesexchange_warp::pagination::List;

const HEADER_ORIGIN_NAME: &str = "Origin";
const HEADER_ORIGIN_VALUE: &str = "waves.exchange";

impl HttpClient<DataService> {
    pub async fn rates<
        I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        S1: AsRef<str>,
    >(
        &self,
        matcher_address: S1,
        pairs: I,
        timestamp: Option<NaiveDateTime>,
    ) -> ApiResult<dto::RatesResponse> {
        let req = dto::RatesRequest {
            pairs: pairs
                .into_iter()
                .map(|(amt, pr)| amt.into() + "/" + &pr.into())
                .collect(),
            timestamp,
        };

        let url = format!("matchers/{}/rates", matcher_address.as_ref());

        self.create_req_handler(
            self.http_post(&url)
                .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE)
                .json(&req),
            "data_service::rates",
        )
        .execute()
        .await
    }

    pub async fn invoke_script_transactions(
        &self,
        senders: Option<impl IntoIterator<Item = impl Into<String>>>,
        timestamp_start: Option<NaiveDateTime>,
        timestamp_end: Option<NaiveDateTime>,
        dapp: Option<impl Into<String>>,
        function: Option<impl Into<String>>,
        after: Option<impl Into<String>>,
        sort: Option<Sort>,
        limit: usize,
    ) -> ApiResult<List<dto::InvokeScriptTransactionResponse>> {
        let senders = senders.map(|s| s.into_iter().map(Into::into).collect::<Vec<_>>());
        let (sender, senders) = if match &senders {
            Some(s) => s.len() == 1,
            None => false,
        } {
            (senders.map(|mut s| s.pop().unwrap()), None)
        } else {
            (None, senders)
        };
        let url = serde_qs::to_string(&InvokeScriptTransactionRequest {
            dapp: dapp.map(Into::into),
            after: after.map(Into::into),
            function: function.map(Into::into),
            limit: if limit == 0 { None } else { Some(limit) },
            sender,
            senders,
            sort,
            timeEnd: timestamp_end.map(Into::into),
            timeStart: timestamp_start.map(Into::into),
        })
        .unwrap();

        self.create_req_handler::<DSList<dto::InvokeScriptTransactionResponse>>(
            self.http_get(format!("transactions/invoke-script?{url}"))
                .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE),
            "data_service::invoke_script_transactions",
        )
        .execute()
        .await
        .map(List::from)
    }

    //TODO Why this fn returns `dto::GenericTransactionResponse`
    // while similar fn `transactions_exchange` returns `dto::ExchangeTransaction`?
    // Is there a real reason for it, or we can use here `dto::ExchangeTransaction` as well?
    pub async fn last_exchange_transaction_to_date(
        &self,
        sender: impl AsRef<str>,
        timestamp: impl Into<NaiveDateTime>,
    ) -> ApiResult<List<dto::GenericTransactionResponse>> {
        let url = format!(
            "transactions/exchange?sender={}&timeEnd={:?}&limit=1",
            sender.as_ref(),
            timestamp.into(),
        );

        self.create_req_handler::<DSList<dto::GenericTransactionResponse>>(
            self.http_get(&url)
                .header(HEADER_ORIGIN_NAME, HEADER_ORIGIN_VALUE),
            "data_service::last_exchange_transaction_to_date",
        )
        .execute()
        .await
        .map(List::from)
    }

    pub async fn asset_by_ticker(
        &self,
        ticker: impl AsRef<str>,
    ) -> ApiResult<dto::Data<Vec<dto::Data<dto::AssetInfo>>>> {
        let url = format!("assets?ticker={}", ticker.as_ref());

        self.create_req_handler(self.http_get(&url), "data_service::asset_by_ticker")
            .execute()
            .await
    }

    //TODO Why this fn returns `dto::ExchangeTransaction`
    // while similar fn `last_exchange_transaction_to_date` returns `dto::GenericTransactionResponse`?
    // Is there a real reason for it?
    pub async fn transactions_exchange(
        &self,
        sender: Option<impl AsRef<str>>,
        amount_asset_id: Option<impl AsRef<str>>,
        price_asset_id: Option<impl AsRef<str>>,
        time_start: Option<DateTime<Utc>>,
        time_end: Option<DateTime<Utc>>,
        sort: Sort,
        limit: usize,
        after: Option<impl AsRef<str>>,
    ) -> ApiResult<List<dto::Data<dto::ExchangeTransaction>>> {
        let query_string = serde_qs::to_string(&dto::ExchangeTransactionsQueryParams {
            amount_asset: amount_asset_id.map(|id| id.as_ref().to_owned()),
            price_asset: price_asset_id.map(|id| id.as_ref().to_owned()),
            sender: sender.map(|id| id.as_ref().to_owned()),
            time_start,
            time_end,
            sort,
            limit,
            after: after.map(|id| id.as_ref().to_owned()),
        })
        .unwrap();

        let url = format!("transactions/exchange?{query_string}");

        self.create_req_handler::<DSList<dto::Data<dto::ExchangeTransaction>>>(
            self.http_get(&url),
            "data_service::transactions_exchange",
        )
        .execute()
        .await
        .map(List::from)
    }

    pub async fn pairs(&self) -> ApiResult<List<dto::Pair>> {
        //FIXME: fetch all pages
        self.create_req_handler::<DSList<_>>(self.http_get("pairs"), "data_service::pairs")
            .execute()
            .await
            .map(List::from)
    }
}

impl<T: Serialize + DeserializeOwned> From<DSList<T>> for List<T> {
    fn from(dsl: DSList<T>) -> Self {
        List::new(dsl.data, !dsl.is_last_page, dsl.last_cursor)
    }
}
