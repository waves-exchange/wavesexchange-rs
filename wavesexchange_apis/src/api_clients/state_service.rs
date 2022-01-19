use self::dto::*;
use crate::{BaseApi, Error, HttpClient};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Instant;
use wavesexchange_log::info;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum HistoryPeg {
    Height(u32),
    Timestamp(String),
}

#[derive(Clone)]
pub struct StateSvcApi;

impl BaseApi for StateSvcApi {}

impl HttpClient<StateSvcApi> {
    pub async fn get_state(
        &self,
        address: impl AsRef<str>,
        key: impl AsRef<str>,
        history_peg: Option<HistoryPeg>,
    ) -> Result<Option<DataEntry>, Error> {
        let key_encoded = utf8_percent_encode(key.as_ref(), NON_ALPHANUMERIC);
        let url = match history_peg {
            None => {
                format!("entries/{}/{}", address.as_ref(), key_encoded,)
            }
            Some(HistoryPeg::Height(height)) => {
                format!(
                    "entries/{}/{}?height={}",
                    address.as_ref(),
                    key_encoded,
                    height,
                )
            }
            Some(HistoryPeg::Timestamp(timestamp)) => {
                format!(
                    "entries/{}/{}?block_timestamp={}",
                    address.as_ref(),
                    key_encoded,
                    timestamp,
                )
            }
        };

        self.create_req_handler(self.get(&url), "state::get_state")
            .handle_status_code(StatusCode::NOT_FOUND, |_| async { Ok(None) })
            .execute()
            .await
    }

    pub async fn search(
        &self,
        query: impl Into<serde_json::Value> + Send,
    ) -> Result<Vec<DataEntry>, Error> {
        let mut entries = vec![];
        let limit = 1000;
        let mut cnt = 0;

        let mut qv: serde_json::Value = query.into();

        qv["limit"] = json!(limit);
        qv["offset"] = json!(0);

        let req_start_time = Instant::now();
        loop {
            let res: StateSearchResult = self
                .create_req_handler(self.post("search").json(&qv), "state::search")
                .execute()
                .await?;

            qv.get_mut("offset")
                .map(|v| *v = (v.as_u64().unwrap() + limit).into());
            cnt += 1;

            entries.extend(res.entries);

            if !res.has_next_page {
                break;
            }
        }

        let req_end_time = Instant::now();
        info!(
            "state search {} requests took {:?} ms",
            cnt,
            (req_end_time - req_start_time).as_millis()
        );

        Ok(entries)
    }
}

pub mod dto {
    use crate::models::DataEntryValue;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct DataEntry {
        pub key: String,
        pub value: DataEntryValue,
        pub address: String,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct StateSearchResult {
        pub entries: Vec<DataEntry>,
        pub has_next_page: bool,
    }
}

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;
    use crate::tests::blockchains::TESTNET;

    pub fn mainnet_client() -> HttpClient<StateSvcApi> {
        HttpClient::from_base_url(MAINNET::state_service_url)
    }

    pub fn testnet_client() -> HttpClient<StateSvcApi> {
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
            .get_state(
                "3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To",
                "%s__priceDecimals",
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entries.key, "%s__priceDecimals");

        let entries_none = testnet_client()
            .get_state("3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To", "%s__priceDeci", None)
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

        let entries = mainnet_client().search(query).await.unwrap();

        assert_eq!(entries.len(), 1);
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

        let entries = mainnet_client().search(query).await.unwrap();

        assert!(entries.len() >= 9);
    }
}
