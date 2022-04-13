mod clients;
mod error;
pub mod test_configs;

pub mod api_clients;
pub mod models;

pub use clients::{grpc::GrpcClient, http::HttpClient, mainnet_client, testnet_client};
pub use error::{ApiResult, Error};

// reexport api structs
pub use api_clients::*;
