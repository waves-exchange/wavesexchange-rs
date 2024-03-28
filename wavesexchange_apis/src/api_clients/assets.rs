use crate::{ApiResult, BaseApi, HttpClient};
use itertools::Itertools;

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
        let mut meta = dto::AssetMetaRequest::default();
        meta.height__gte = height;
        meta.format = format.to_option();
        meta.include_metadata = include_metadata;
        let meta = serde_qs::to_string(&meta).expect("query string");

        let body = dto::AssetRequest { ids };

        self.create_req_handler(
            self.http_post(format!("?{meta}")).json(&body),
            "assets::get_assets",
        )
        .execute()
        .await
    }

    /// Create new Asset Service search request builder.
    ///
    /// Example:
    /// ```no_run
    /// # use wavesexchange_apis::{HttpClient, AssetsService, assets::dto::{AssetLabel, OutputFormat}};
    /// let assets_client: HttpClient<AssetsService> /* = ... */;
    /// # assets_client = HttpClient::new();
    /// # tokio_test::block_on(async {
    /// let response = assets_client
    ///     .new_search()
    ///     .with_format(OutputFormat::Full)
    ///     .with_metadata(true)
    ///     .with_labels(&[AssetLabel::Stablecoin, AssetLabel::Gateway])
    ///     .search()
    ///     .await;
    /// # });
    /// ```
    #[inline]
    pub fn new_search(&self) -> request::Builder {
        request::Builder::new(self)
    }

    #[inline]
    async fn search(&self, req: request::Builder<'_>) -> ApiResult<dto::AssetResponse> {
        if let Some(ref ids) = req.ids {
            if ids.is_empty() {
                return Ok(dto::AssetResponse {
                    data: vec![],
                    cursor: None,
                });
            }
        }

        let meta = dto::AssetMetaRequest {
            height__gte: req.height,
            format: req.format.to_option(),
            include_metadata: req.include_metadata,
            search: req.search,
            ticker: req.ticker,
            ext_ticker: req.ext_ticker,
            smart: req.smart,
            label: req.label,
            label__in: req.labels.map(|set| set.into_iter().collect_vec()),
            issuer__in: req.issuers.map(|set| set.into_iter().collect_vec()),
            limit: req.limit,
            after: req.after,
        };
        let meta = serde_qs::to_string(&meta).expect("query string");

        let body = req.ids.map(|ids| dto::AssetRequest { ids });

        let request_builder = if let Some(body) = body {
            self.http_post(format!("?{meta}")).json(&body)
        } else {
            self.http_get(format!("?{meta}"))
        };
        self.create_req_handler(request_builder, "assets::get_assets")
            .execute()
            .await
    }
}

pub mod request {
    use super::{dto, AssetsService};
    use crate::{ApiResult, HttpClient};
    use std::collections::HashSet;

    #[derive(Clone, Debug)]
    pub struct Builder<'a> {
        client: Option<&'a HttpClient<AssetsService>>,

        /// Output format: brief or full. Default is brief.
        pub(super) format: dto::OutputFormat,
        /// Whether to include metadata from oracles. Default is false.
        pub(super) include_metadata: bool,

        /// Search string. Default is None.
        pub(super) search: Option<String>,
        /// Asset ids to locate. Default is None.
        pub(super) ids: Option<Vec<String>>,
        /// Ticker value or `*` for any asset having ticker value. Default is None.
        pub(super) ticker: Option<String>,
        /// External ticker value or `*` for any asset having external ticker value. Default is None.
        pub(super) ext_ticker: Option<String>,
        /// Asset labels contain label value or `*` for assets having any label. Default is None.
        pub(super) label: Option<String>,
        /// Asset labels to query. Default is None.
        pub(super) labels: Option<HashSet<dto::AssetLabel>>,
        /// Asset issuer address (base58 string) filter. Default is None.
        pub(super) issuers: Option<HashSet<String>>,
        /// Smart asset flag value. Default is None.
        pub(super) smart: Option<bool>,
        /// Response with assets at height (greater or equal) (available only with `ids` filter as of now).
        pub(super) height: Option<u32>,

        /// Output limit. Default is None.
        pub(super) limit: Option<u32>,
        /// Cursor value to query for the next page as returned from previous page search. Default is None.
        pub(super) after: Option<String>,
    }

    impl<'a> Builder<'a> {
        pub(super) fn new(client: &'a HttpClient<AssetsService>) -> Self {
            Builder {
                client: Some(client),
                format: dto::OutputFormat::Brief,
                include_metadata: false,
                search: None,
                ids: None,
                ticker: None,
                ext_ticker: None,
                label: None,
                labels: None,
                issuers: None,
                smart: None,
                height: None,
                limit: None,
                after: None,
            }
        }

        /// Output format: brief or full. Default is brief.
        pub fn with_format(mut self, format: dto::OutputFormat) -> Self {
            self.format = format;
            self
        }

        /// Whether to include metadata from oracles. Default is false.
        pub fn with_metadata(mut self, metadata: bool) -> Self {
            self.include_metadata = metadata;
            self
        }

        /// Search string. Default is None.
        pub fn with_search_string(mut self, search_string: impl Into<String>) -> Self {
            self.search = Some(search_string.into());
            self
        }

        /// Asset ids to locate. Default is None.
        pub fn with_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
            self.ids = Some(ids.into_iter().map(Into::into).collect());
            self
        }

        /// Ticker value or `*` for any asset having ticker value. Default is None.
        pub fn with_ticker(mut self, ticker: impl Into<String>) -> Self {
            self.ticker = Some(ticker.into());
            self
        }

        /// External ticker value or `*` for any asset having external ticker value. Default is None.
        pub fn with_ext_ticker(mut self, ext_ticker: impl Into<String>) -> Self {
            self.ext_ticker = Some(ext_ticker.into());
            self
        }

        /// Asset labels contain label value or `*` for assets having any label. Default is None.
        pub fn with_label(mut self, label: impl Into<String>) -> Self {
            self.label = Some(label.into());
            self
        }

        /// Asset labels to query. Default is None.
        pub fn with_labels(mut self, labels: &[dto::AssetLabel]) -> Self {
            self.labels = Some(labels.iter().cloned().collect());
            self
        }

        /// Asset issuer address (base58 string) filter. Default is None.
        pub fn with_issuers(mut self, issuers: impl IntoIterator<Item = impl Into<String>>) -> Self {
            self.issuers = Some(issuers.into_iter().map(Into::into).collect());
            self
        }

        /// Smart asset flag value. Default is None.
        pub fn with_smart(mut self, smart: bool) -> Self {
            self.smart = Some(smart);
            self
        }

        /// Response with assets at height (greater or equal) (available only with `ids` filter as of now).
        pub fn with_height(mut self, height: u32) -> Self {
            self.height = Some(height);
            self
        }

        /// Output limit. Default is None.
        pub fn with_limit(mut self, limit: u32) -> Self {
            self.limit = Some(limit);
            self
        }

        /// Cursor value to query for the next page as returned from previous page search.
        pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
            self.after = cursor;
            self
        }

        /// Perform the search.
        pub async fn search(mut self) -> ApiResult<dto::AssetResponse> {
            let client = self.client.take().expect("http_client");
            client.search(self).await
        }
    }
}

pub mod dto {
    use crate::models::dto::DataEntryValue;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
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
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct OracleData(pub HashMap<String, DataEntryValue>);

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AssetLabel {
        Gateway,
        #[serde(rename = "DEFI")]
        DeFi,
        Stablecoin,
        Qualified,
        WaVerified,
        CommunityVerified,
        #[serde(rename = "WX")]
        WX,
        #[serde(rename = "3RD_PARTY")]
        ThirdParty,
        Pepe,
        #[serde(rename = "STAKING_LP")]
        StakingLP,
        #[serde(rename = "ALGO_LP")]
        AlgoLP,
        #[serde(rename = "POOLS_LP")]
        PoolsLP,
        #[serde(rename = "null")]
        WithoutLabels,
        #[serde(other)]
        Other,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct FullAssetInfo {
        pub ticker: Option<String>,
        pub ext_ticker: Option<String>,
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

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum OutputFormat {
        Brief,
        Full,
        None,
    }

    impl OutputFormat {
        pub(super) fn to_option(self) -> Option<Self> {
            if self == OutputFormat::None {
                None
            } else {
                Some(self)
            }
        }
    }

    #[allow(non_snake_case)]
    #[derive(Default, Debug, Serialize)]
    pub(super) struct AssetMetaRequest {
        pub height__gte: Option<u32>,
        pub format: Option<OutputFormat>,
        pub include_metadata: bool,
        pub search: Option<String>,
        pub ticker: Option<String>,
        pub ext_ticker: Option<String>,
        pub smart: Option<bool>,
        pub label: Option<String>,
        pub label__in: Option<Vec<AssetLabel>>,
        pub issuer__in: Option<Vec<String>>,
        pub limit: Option<u32>,
        pub after: Option<String>,
    }

    #[derive(Debug, Serialize)]
    pub(super) struct AssetRequest {
        pub ids: Vec<String>,
    }
}
