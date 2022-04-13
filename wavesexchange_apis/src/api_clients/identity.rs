use crate::{ApiResult, BaseApi, HttpClient};

const AUTHORIZATION_HEADER: &str = "Authorization";

#[derive(Clone, Debug)]
pub struct Identity;

impl BaseApi for Identity {
    const MAINNET_URL: &'static str = "https://id.waves.exchange/";
    const TESTNET_URL: &'static str = "https://id-testnet.waves.exchange/";
}

impl HttpClient<Identity> {
    pub async fn sign(
        &self,
        access_token: String,
        payload: String,
        signature: String,
    ) -> ApiResult<dto::SignResponse> {
        let req = dto::SignRequest { payload, signature };
        self.create_req_handler(
            self.http_post("/api/v1/sign")
                .json(&req)
                .header(AUTHORIZATION_HEADER, access_token),
            "identity::sign",
        )
        .execute()
        .await
    }
}

pub mod dto {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize)]
    pub(super) struct SignRequest {
        pub payload: String,
        pub signature: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct SignResponse {
        pub signature: String,
    }
}
