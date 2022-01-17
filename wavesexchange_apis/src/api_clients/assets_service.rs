use crate::{BaseApi, Error, HttpClient};
use itertools::join;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::sync::Arc;
use wavesexchange_log::{timer, trace};

#[derive(Clone)]
pub struct AssetsSvcApi(Box<HttpClient<Self>>);

impl BaseApi for AssetsSvcApi {
    fn new_http(cli: &HttpClient<Self>) -> Self {
        AssetsSvcApi(Box::new(cli.clone()))
    }
}

pub struct AssetInfo {
    pub id: String,
    pub quantity: i64,
}

impl AssetsSvcApi {
    pub async fn get_assets<S, I>(
        &self,
        asset_ids: I,
        height: Option<u32>,
    ) -> Result<Vec<AssetInfo>, Error>
    where
        S: AsRef<str> + Send,
        I: IntoIterator<Item = S> + Send,
    {
        let url = build_url(&self.0.base_url(), asset_ids, height);
        if url.is_none() {
            return Ok(vec![]);
        }
        let url = url.unwrap();
        trace!("AssetsService url: {}", url);

        timer!("AssetService query");

        let resp = self
            .0
            .get_client()
            .get(&url)
            .send()
            .await
            .map_err(|err| {
                Error::HttpRequestError(Arc::new(err), "Failed to query Assets Service".to_string())
            })?
            .json::<dto::AssetResponse>()
            .await
            .map_err(|err| {
                Error::HttpRequestError(
                    Arc::new(err),
                    "Failed to parse json response from Assets Service".to_string(),
                )
            })?;

        let res = resp
            .data
            .into_iter()
            .map(|asset_data| AssetInfo {
                id: asset_data.data.id,
                quantity: asset_data.data.quantity,
            })
            .collect();

        Ok(res)
    }
}

pub mod dto {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(super) struct AssetResponse {
        pub data: Vec<AssetData>,
    }

    #[derive(Deserialize)]
    pub(super) struct AssetData {
        pub data: Asset,
    }

    #[derive(Deserialize)]
    pub(super) struct Asset {
        pub id: String,
        pub quantity: i64,
    }
}

fn build_url<S, I>(root_url: &str, asset_ids: I, height: Option<u32>) -> Option<String>
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    let asset_ids = asset_ids
        .into_iter()
        .map(|id| utf8_percent_encode(id.as_ref(), NON_ALPHANUMERIC).to_string());
    let ids = join(asset_ids, "&ids=");
    if ids.is_empty() {
        return None;
    }
    let mut url = format!("{}?ids={}", root_url, ids);
    if let Some(height) = height {
        url.push_str("&height__gte=");
        url.push_str(&height.to_string());
    }
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        assert_eq!(build_url::<&str, _>("http://assets", vec![], None), None);
        assert_eq!(build_url::<&str, _>("http://assets", vec![], Some(1)), None);
        assert_eq!(
            build_url("http://assets", vec!["123"], None).unwrap(),
            "http://assets?ids=123"
        );
        assert_eq!(
            build_url("http://assets", vec!["123", "456"], None).unwrap(),
            "http://assets?ids=123&ids=456"
        );
        assert_eq!(
            build_url("http://assets", vec!["123", "456"], Some(789)).unwrap(),
            "http://assets?ids=123&ids=456&height__gte=789"
        );
        assert_eq!(
            build_url("http://assets", vec!["foo%"], None).unwrap(),
            "http://assets?ids=foo%25"
        );
    }
}
