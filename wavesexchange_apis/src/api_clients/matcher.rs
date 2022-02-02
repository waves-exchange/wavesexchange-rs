use crate::{ApiResult, BaseApi, HttpClient};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MatcherApi;

impl BaseApi for MatcherApi {}

impl HttpClient<MatcherApi> {
    pub async fn get(&self) -> ApiResult<HashMap<String, BigDecimal>> {
        self.create_req_handler(self.http_get(""), "matcher::get")
            .execute()
            .await
    }

    pub async fn orderbook(&self, order: String) -> ApiResult<dto::PlaceOrderResponse> {
        self.create_req_handler(
            self.http_post("matcher/orderbook")
                .header("Content-Type", "application/json")
                .body(order.into_bytes()),
            "matcher::orderbook",
        )
        .execute()
        .await
    }

    pub async fn orderbook_market(&self, order: String) -> ApiResult<dto::PlaceOrderResponse> {
        self.create_req_handler(
            self.http_post("matcher/orderbook/market")
                .header("Content-Type", "application/json")
                .body(order.into_bytes()),
            "matcher::orderbook_market",
        )
        .execute()
        .await
    }
}

pub mod dto {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub enum OrderStatus {
        OrderAccepted,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct PlaceOrderResponse {
        pub success: bool,
        pub status: OrderStatus,
        pub message: serde_json::Value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::blockchains::TESTNET;

    #[tokio::test]
    async fn test_assets_from_matcher() {
        let client = HttpClient::<MatcherApi>::from_base_url(TESTNET::matcher_api_url);
        let resp = client.get().await.unwrap();
        assert_eq!(resp["WAVES"], BigDecimal::from(1));
    }
}
