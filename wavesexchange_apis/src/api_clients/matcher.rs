use crate::{BaseApi, Error, HttpClient};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MatcherApi;

impl BaseApi for MatcherApi {}

impl HttpClient<MatcherApi> {
    pub async fn assets_from_matcher(
        &self,
        url: &String,
    ) -> Result<Option<HashMap<String, BigDecimal>>, Error> {
        let req = self.get("/");
        self.do_request(req, "assets from matcher").await
    }
}
