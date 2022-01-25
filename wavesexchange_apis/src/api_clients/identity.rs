use crate::{ApiResult, BaseApi, Error, HttpClient};

const AUTHORIZATION_HEADER: &str = "Authorization";

#[derive(Clone, Debug)]
pub struct IdentityApi;

impl BaseApi for IdentityApi {}

impl HttpClient<IdentityApi> {
    pub async fn sign(
        &self,
        access_token: String,
        payload: String,
        signature: String,
    ) -> ApiResult<String> {
        let req = dto::SignRequest { payload, signature };

        let resp: dto::SignResponse = self
            .create_req_handler(
                self.post("/api/v1/sign")
                    .json(&req)
                    .header(AUTHORIZATION_HEADER, access_token),
                "identity::sign",
            )
            .execute()
            .await?;

        let signature_bytes =
            base64::decode(resp.signature).map_err(|e| Error::ResponseParseError(e.to_string()))?;
        let signature_base58 = bs58::encode(signature_bytes).into_string();
        Ok(signature_base58)
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
    pub(super) struct SignResponse {
        pub signature: String,
    }
}
