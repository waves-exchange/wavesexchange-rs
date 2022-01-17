mod clients;
mod error;
#[cfg(test)]
mod tests;

pub mod api_clients;
pub mod models;

pub use clients::{grpc::GrpcClient, http::HttpClient};
pub use error::Error;

// reexport api traits
pub use api_clients::*;
