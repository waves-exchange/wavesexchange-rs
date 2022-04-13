pub mod grpc;
pub mod http;

use crate::BaseApi;
use http::HttpClient;

pub fn mainnet_client<A: BaseApi>() -> HttpClient<A> {
    HttpClient::from_base_url(A::MAINNET_URL)
}

pub fn testnet_client<A: BaseApi>() -> HttpClient<A> {
    HttpClient::from_base_url(A::TESTNET_URL)
}
