use crate::{ApiResult, BaseApi};
use std::{marker::PhantomData, sync::Arc};
use waves_protobuf_schemas::tonic;

pub use waves_protobuf_schemas::waves::events::grpc::blockchain_updates_api_client::BlockchainUpdatesApiClient;

#[derive(Clone, Debug)]
pub struct GrpcClient<A: BaseApi> {
    pub grpc_client: BlockchainUpdatesApiClient<tonic::transport::Channel>,
    _pd: PhantomData<A>,
}

impl<A: BaseApi> GrpcClient<A> {
    pub async fn new(blockchain_updates_url: &str) -> ApiResult<Self> {
        Ok(GrpcClient {
            grpc_client: BlockchainUpdatesApiClient::connect(blockchain_updates_url.to_owned())
                .await
                .map_err(Arc::new)?,
            _pd: PhantomData,
        })
    }
}
