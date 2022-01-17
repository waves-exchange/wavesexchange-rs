use crate::{BaseApi, Error};
use std::{ops::Deref, sync::Arc};
use waves_protobuf_schemas::waves::events::grpc::blockchain_updates_api_client::BlockchainUpdatesApiClient;

#[derive(Clone)]
pub struct GrpcClient<A: BaseApi<Self>> {
    pub grpc_client: BlockchainUpdatesApiClient<tonic::transport::Channel>,
    api: Option<A>,
}

impl<A: BaseApi<Self>> GrpcClient<A> {
    pub async fn new(blockchain_updates_url: &str) -> Result<Self, Error> {
        let mut client = GrpcClient {
            grpc_client: BlockchainUpdatesApiClient::connect(blockchain_updates_url.to_owned())
                .await
                .map_err(Arc::new)?,
            api: None,
        };
        client.api = Some(A::new(&client));
        Ok(client)
    }
}

impl<A: BaseApi<Self>> Deref for GrpcClient<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.api.as_ref().unwrap()
    }
}
