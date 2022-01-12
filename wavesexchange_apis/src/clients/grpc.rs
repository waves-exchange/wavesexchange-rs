use crate::Error;
use std::sync::Arc;
use waves_protobuf_schemas::waves::events::grpc::blockchain_updates_api_client::BlockchainUpdatesApiClient;

#[derive(Clone)]
pub struct GrpcClient {
    pub grpc_client: BlockchainUpdatesApiClient<tonic::transport::Channel>,
}

impl GrpcClient {
    pub async fn new(blockchain_updates_url: &str) -> Result<Self, Error> {
        Ok(GrpcClient {
            grpc_client: BlockchainUpdatesApiClient::connect(blockchain_updates_url.to_owned())
                .await
                .map_err(Arc::new)?,
        })
    }
}
