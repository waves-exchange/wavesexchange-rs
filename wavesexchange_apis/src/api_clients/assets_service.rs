use crate::{BaseApi, Error, HttpClient};
use itertools::join;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use wavesexchange_log::timer;

#[derive(Clone)]
pub struct AssetsSvcApi;

impl BaseApi for AssetsSvcApi {}

pub struct AssetInfo {
    pub id: String,
    pub quantity: i64,
}

impl HttpClient<AssetsSvcApi> {
    pub async fn get_assets<S, I>(
        &self,
        asset_ids: I,
        height: Option<u32>,
    ) -> Result<Vec<AssetInfo>, Error>
    where
        S: AsRef<str> + Send,
        I: IntoIterator<Item = S> + Send,
    {
        let url = match build_url(&self.base_url(), asset_ids, height) {
            Some(u) => u,
            None => return Ok(vec![]),
        };

        timer!("AssetService query");

        let resp: dto::AssetResponse = self
            .create_req_handler(self.get_client().get(&url), "assets::get_assets")
            .execute()
            .await?;

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
