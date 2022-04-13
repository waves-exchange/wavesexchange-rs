use crate::{ApiResult, BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct AssetsService;

impl BaseApi for AssetsService {
    const MAINNET_URL: &'static str = "https://waves.exchange/api/v1/assets";
    const TESTNET_URL: &'static str = "https://testnet.waves.exchange/api/v1/assets";
}

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
