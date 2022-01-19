use crate::{BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MatcherApi;

impl BaseApi for MatcherApi {}

impl HttpClient<MatcherApi> {
    pub async fn assets_from_matcher(&self) -> Result<Option<HashMap<String, BigDecimal>>, Error> {
        self.create_req_handler(self.get("/"), "matcher::assets_from_matcher")
            .execute()
            .await
    }
}
