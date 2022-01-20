use crate::{BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MatcherApi;

impl BaseApi for MatcherApi {}

impl HttpClient<MatcherApi> {
    pub async fn assets_from_matcher(&self) -> Result<HashMap<String, BigDecimal>, Error> {
        self.create_req_handler(self.get(""), "matcher::assets_from_matcher")
            .execute()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::blockchains::TESTNET;

    #[tokio::test]
    async fn test_assets_from_matcher() {
        let client = HttpClient::<MatcherApi>::from_base_url(TESTNET::matcher_api_url);
        let resp = client.assets_from_matcher().await.unwrap();
        assert_eq!(resp["WAVES"], BigDecimal::from(1));
    }
}
