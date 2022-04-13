use crate::{ApiResult, BaseApi, HttpClient};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Matcher;

impl BaseApi for Matcher {
    const MAINNET_URL: &'static str = "https://matcher.waves.exchange/matcher/settings/rates";
    const TESTNET_URL: &'static str =
        "https://matcher-testnet.waves.exchange/matcher/settings/rates";
}

impl HttpClient<Matcher> {
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
