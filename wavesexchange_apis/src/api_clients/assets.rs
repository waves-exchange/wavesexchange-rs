use crate::{ApiResult, BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct AssetsService;

impl BaseApi for AssetsService {}

impl HttpClient<AssetsService> {
    pub async fn get(
        &self,
        asset_ids: impl IntoIterator<Item = impl Into<String>>,
        height: Option<u32>,
        format: dto::OutputFormat,
        include_metadata: bool,
    ) -> ApiResult<dto::AssetResponse> {
        let ids = asset_ids.into_iter().map(Into::into).collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(dto::AssetResponse {
                data: vec![],
                cursor: None,
            });
        }
        let url = serde_qs::to_string(&dto::AssetRequest {
            ids,
            height__gte: height,
            format,
            include_metadata,
        })
        .unwrap();

        self.create_req_handler(self.http_get(format!("?{url}")), "assets::get_assets")
            .execute()
            .await
    }
}

pub mod dto {
    use crate::models::dto::DataEntryValue;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use serde_repr::{Deserialize_repr, Serialize_repr};
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    pub struct AssetResponse {
        pub data: Vec<AssetData>,
        pub cursor: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(tag = "type", rename = "asset")]
    pub struct AssetData {
        pub data: Option<AssetInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub metadata: Option<AssetMetadata>,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(untagged)]
    pub enum AssetInfo {
        Full(FullAssetInfo),
        Brief(BriefAssetInfo),
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct AssetMetadata {
        pub oracle_data: Vec<OracleData>,
        pub labels: Vec<AssetLabel>,
        pub sponsor_balance: Option<i64>,
        pub has_image: bool,
        pub verified_status: VerificationStatus,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct OracleData(HashMap<String, DataEntryValue>);

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AssetLabel {
        Gateway,
        #[serde(rename = "DEFI")]
        DeFi,
        Stablecoin,
        Qualified,
        WaVerified,
        CommunityVerified,
        #[serde(rename = "null")]
        WithoutLabels,
    }

    #[derive(Clone, Debug, Serialize_repr, Deserialize_repr)]
    #[repr(i8)]
    pub enum VerificationStatus {
        Verified = 1,
        Unknown = 0,
        Declined = -1,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct FullAssetInfo {
        pub ticker: Option<String>,
        pub id: String,
        pub name: String,
        pub precision: i32,
        pub description: String,
        pub height: i32,
        pub timestamp: DateTime<Utc>,
        pub sender: String,
        pub quantity: i64,
        pub reissuable: bool,
        pub has_script: bool,
        pub min_sponsored_fee: Option<i64>,
        pub smart: bool,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct BriefAssetInfo {
        pub ticker: Option<String>,
        pub id: String,
        pub name: String,
        pub smart: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct Asset {
        pub id: String,
        pub quantity: i64,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum OutputFormat {
        Brief,
        Full,
        None,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Serialize)]
    pub struct AssetRequest {
        pub ids: Vec<String>,
        pub height__gte: Option<u32>,
        pub format: OutputFormat,
        pub include_metadata: bool,
    }
}

// public exports for tests
pub mod tests {
    use super::*;
    use crate::tests::blockchains::MAINNET;

    pub fn mainnet_client() -> HttpClient<AssetsService> {
        HttpClient::from_base_url(MAINNET::assets_service_url)
    }
}

#[cfg(test)]
mod tests_internal {
    use super::tests::*;
    use super::*;

    #[tokio::test]
    async fn test_assets_get() {
        let resp = mainnet_client()
            .get(vec!["WAVES"], Some(1), dto::OutputFormat::Full, true)
            .await
            .unwrap();
        let resp = &resp.data[0];
        let data = if let dto::AssetInfo::Full(r) = resp.data.as_ref().unwrap() {
            r
        } else {
            panic!("Wrong output format");
        };
        assert_eq!(&data.id, "WAVES");
        assert_eq!(data.quantity, 10000000000000000);
        let label = &resp.metadata.as_ref().expect("no metadata found").labels[0];
        assert!(matches!(label, dto::AssetLabel::Gateway));
    }
}
