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

impl BaseApi for StateService {}

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

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;
    use crate::tests::blockchains::TESTNET;

    pub fn mainnet_client() -> HttpClient<StateService> {
        HttpClient::from_base_url(MAINNET::state_service_url)
    }

    pub fn testnet_client() -> HttpClient<StateService> {
        HttpClient::from_base_url(TESTNET::state_service_url)
    }
}

#[cfg(test)]
mod tests_internal {
    use super::tests::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_state() {
        let entries = testnet_client()
            .entries(
                "3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To",
                "%s__priceDecimals",
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entries.key, "%s__priceDecimals");

        let entries_none = testnet_client()
            .entries("3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To", "%s__priceDeci", None)
            .await
            .unwrap();
        assert!(entries_none.is_none());
    }

    #[tokio::test]
    async fn single_asset_price_request() {
        let query = json!({
            "filter": {
                "in": {
                    "properties": [
                        {
                            "address": {}
                        },
                        {
                            "key": {}
                        }
                    ],
                    "values": [
                        ["3P8qJyxUqizCWWtEn2zsLZVPzZAjdNGppB1", "%s%s__price__UAH"]
                    ]
                }
            }
        });

        let entries = mainnet_client().search(query, None, None).await.unwrap();

        assert_eq!(entries.items.len(), 1);
    }

    #[tokio::test]
    async fn defo_assets_list() {
        let query = json!({
            "filter": {
                "and": [
                  {
                    "address": {
                      "value": "3PQEjFmdcjd6wf1TrpkHSuDAk3zbfLSeikb"
                    }
                  },
                  {
                    "fragment": {
                      "position": 0,
                      "type": "string",
                      "operation": "eq",
                      "value": "defoAsset"
                    }
                  },
                  {
                    "fragment": {
                      "position": 2,
                      "type": "string",
                      "operation": "eq",
                      "value": "config"
                    }
                  }
                ]
            }
        });

        let entries = mainnet_client().search(query, None, None).await.unwrap();

        assert!(entries.items.len() >= 9);
    }
}
