use crate::{ApiResult, BaseApi, HttpClient};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::StatusCode;
use serde_json::json;
use wavesexchange_warp::pagination::List;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum HistoryQuery {
    Height(u32),
    Timestamp(String),
}

#[derive(Clone, Debug)]
pub struct StateService;

impl BaseApi for StateService {
    const MAINNET_URL: &'static str = "https://waves.exchange/api/v1/state";
    const TESTNET_URL: &'static str = "https://testnet.waves.exchange/api/v1/state";
}

impl HttpClient<StateService> {
    pub async fn entries(
        &self,
        address: impl AsRef<str>,
        key: impl AsRef<str>,
        history_query: Option<HistoryQuery>,
    ) -> ApiResult<Option<dto::DataEntry>> {
        let key_encoded = utf8_percent_encode(key.as_ref(), NON_ALPHANUMERIC);
        let url = match history_query {
            None => {
                format!("entries/{}/{}", address.as_ref(), key_encoded,)
            }
            Some(HistoryQuery::Height(height)) => {
                format!(
                    "entries/{}/{}?height={}",
                    address.as_ref(),
                    key_encoded,
                    height,
                )
            }
            Some(HistoryQuery::Timestamp(timestamp)) => {
                format!(
                    "entries/{}/{}?block_timestamp={}",
                    address.as_ref(),
                    key_encoded,
                    timestamp,
                )
            }
        };

        self.create_req_handler(self.http_get(&url), "state::entries")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn search(
        &self,
        query: impl Into<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> ApiResult<List<dto::DataEntry>> {
        let mut entries = vec![];
        let limit = limit.unwrap_or(1000);
        let offset = offset.unwrap_or(0);

        let mut qv: serde_json::Value = query.into();
        qv["limit"] = json!(limit);
        qv["offset"] = json!(offset);

        loop {
            let res: List<dto::DataEntry> = self
                .create_req_handler::<dto::StateSearchResult, _>(
                    self.http_post("search").json(&qv),
                    "state::search",
                )
                .execute()
                .await
                .map(List::from)?;

            qv.get_mut("offset")
                .map(|v| *v = (v.as_u64().unwrap() + limit).into());

            entries.extend(res.items);

            if !res.page_info.has_next_page {
                return Ok(List {
                    page_info: res.page_info,
                    items: entries,
                });
            }
        }
    }
}

pub mod dto {
    pub use crate::models::dto::{DataEntry, DataEntryValue};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct StateSearchResult {
        pub entries: Vec<DataEntry>,
        pub has_next_page: bool,
    }
}

impl From<dto::StateSearchResult> for List<dto::DataEntry> {
    fn from(ssr: dto::StateSearchResult) -> Self {
        List::new(ssr.entries, ssr.has_next_page, None)
    }
}
