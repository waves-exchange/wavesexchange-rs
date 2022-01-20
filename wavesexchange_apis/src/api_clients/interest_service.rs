use self::dto::*;
use crate::{BaseApi, Error, HttpClient};

#[derive(Clone, Debug)]
pub struct InterestSvcApi;

impl BaseApi for InterestSvcApi {}

impl HttpClient<InterestSvcApi> {
    pub async fn interest_rates(
        &self,
        asset_id: impl AsRef<str>,
    ) -> Result<Vec<AnnualRate>, Error> {
        let url = format!("interest_rates/{}", asset_id.as_ref());

        let resp: InterestRatesResponse = self
            .create_req_handler(self.get(&url), "interest::interest_rates")
            .execute()
            .await?;
        Ok(resp.annual_rates)
    }
}

#[allow(dead_code)]
pub mod dto {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub(super) struct InterestRatesResponse {
        pub asset_id: String,
        pub annual_rates: Vec<AnnualRate>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AnnualRate {
        pub income_type: String,
        pub rate: f64,
    }
}
