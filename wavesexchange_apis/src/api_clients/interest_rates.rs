use crate::{ApiResult, BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct InterestSvcApi;

impl BaseApi for InterestSvcApi {}

impl HttpClient<InterestSvcApi> {
    pub async fn get(&self, asset_id: impl AsRef<str>) -> ApiResult<dto::InterestRatesResponse> {
        let url = format!("interest_rates/{}", asset_id.as_ref());

        self.create_req_handler(self.http_get(&url), "interest_rates::get")
            .execute()
            .await
    }
}

#[allow(dead_code)]
pub mod dto {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct InterestRatesResponse {
        pub asset_id: String,
        pub annual_rates: Vec<AnnualRate>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AnnualRate {
        pub income_type: String,
        pub rate: f64,
    }
}
