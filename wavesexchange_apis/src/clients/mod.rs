pub mod grpc;
pub mod http;

/*
use crate::BaseApi;
pub trait ApiClient {}

impl<A: BaseApi<Self>> ApiClient for http::HttpClient<A> {}
impl<A: BaseApi<Self>> ApiClient for grpc::GrpcClient<A> {}
*/
