use crate::{ApiResult, BaseApi, HttpClient};
use async_recursion::async_recursion;
use wavesexchange_warp::pagination::List;

#[derive(Clone, Debug)]
pub struct TransfersApi;

impl BaseApi for TransfersApi {}

impl HttpClient<TransfersApi> {
    #[async_recursion]
    pub async fn get(
        &self,
        req: dto::SearchTransfersRequest,
    ) -> ApiResult<List<dto::TransferResponse>> {
        let request_url = format!("transfers?{}", serde_qs::to_string(&req).unwrap());

        self.create_req_handler(self.http_get(&request_url), "transfers::get")
            .execute()
            .await
    }
}

pub mod dto {
    use chrono::{DateTime, FixedOffset, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum TxType {
        MassTransfer,
        Payment,
        Transfer,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TransferResponse {
        pub origin_transaction_type: TxType,
        pub sender: String,
        pub block_timestamp: Option<DateTime<FixedOffset>>,
        pub recipient: Option<String>,
        pub amount: i64,
        pub asset_id: String,
        pub attachment: Option<String>,
        pub attachment_utf8: Option<String>,
    }

    #[allow(non_snake_case)]
    #[derive(Clone, Debug, Serialize)]
    pub struct SearchTransfersRequest {
        pub sender__in: Option<Vec<String>>,
        pub recipient: Option<String>,
        pub asset_id: Option<String>,
        pub asset_id__in: Option<Vec<String>>,
        pub attachment_utf8__match: Option<String>,
        pub block_timestamp__gte: Option<DateTime<Utc>>,
        pub block_timestamp__lt: Option<DateTime<Utc>>,
        pub limit: Option<i64>,
        pub after: Option<String>,
    }
}
