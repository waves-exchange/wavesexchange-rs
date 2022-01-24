use crate::{ApiResult, BaseApi, HttpClient};
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use wavesexchange_warp::pagination::List;

#[derive(Clone, Debug)]
pub struct TransfersApi;

impl BaseApi for TransfersApi {}

impl HttpClient<TransfersApi> {
    #[async_recursion]
    pub async fn search(&self, req: SearchTransfersRequest) -> ApiResult<Vec<Transfer>> {
        let mut query_params = vec![];

        if let Some(senders) = &req.senders {
            query_params.push(
                senders
                    .iter()
                    .map(|sender| format!("sender__in[]={}", sender))
                    .collect::<Vec<String>>()
                    .join("&"),
            );
        }

        if let Some(recipient) = &req.recipient {
            query_params.push(format!("recipient={}", recipient));
        }

        if let Some(asset_id) = &req.asset_id {
            query_params.push(format!("asset_id={}", asset_id));
        }

        if let Some(asset_ids) = &req.asset_id_in {
            query_params.push(
                asset_ids
                    .iter()
                    .map(|asset_id| format!("asset_id__in[]={}", asset_id))
                    .collect::<Vec<String>>()
                    .join("&"),
            );
        }

        if let Some(attachment_utf8_match) = &req.attachment_utf8_match {
            query_params.push(format!("attachment_utf8__match={}", attachment_utf8_match));
        }

        if let Some(block_timestamp_gte) = &req.block_timestamp_gte {
            query_params.push(format!("block_timestamp__gte={}", block_timestamp_gte));
        }

        if let Some(block_timestamp_lt) = &req.block_timestamp_lt {
            query_params.push(format!("block_timestamp__lt={}", block_timestamp_lt));
        }

        if let Some(limit) = &req.limit {
            query_params.push(format!("limit={}", limit));
        }

        if let Some(after) = &req.after {
            query_params.push(format!("after={}", after));
        }

        let request_url = format!("transfers?{}", query_params.join("&"));

        let response: List<dto::TransferResponse> = self
            .create_req_handler(self.get(&request_url), "transfers::search")
            .execute()
            .await?;

        let mut transfers: Vec<Transfer> =
            response.clone().items.iter().map(|t| t.into()).collect();

        if transfers.len() < req.limit.unwrap_or(0) as usize && response.page_info.has_next_page {
            let mut req = req.clone();
            req.after = response.page_info.last_cursor;
            let mut rest_transfers = self.search(req).await?;
            transfers.append(&mut rest_transfers);
        }

        Ok(transfers)
    }
}

#[derive(Clone, Debug)]
pub struct Transfer {
    pub asset_id: String,
    pub amount: i64,
    pub attachment_utf8: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SearchTransfersRequest {
    pub senders: Option<Vec<String>>,
    pub recipient: Option<String>,
    pub asset_id: Option<String>,
    pub asset_id_in: Option<Vec<String>>,
    pub attachment_utf8_match: Option<String>,
    pub block_timestamp_gte: Option<DateTime<Utc>>,
    pub block_timestamp_lt: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub after: Option<String>,
}

pub mod dto {
    use chrono::{DateTime, FixedOffset};
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
}

impl From<&dto::TransferResponse> for Transfer {
    fn from(t: &dto::TransferResponse) -> Self {
        Transfer {
            asset_id: t.asset_id.to_owned(),
            amount: t.amount,
            attachment_utf8: t.attachment_utf8.to_owned(),
        }
    }
}
