mod client;
mod error;
#[cfg(test)]
mod tests;

pub mod api_clients;
pub mod models;

#[macro_use]
extern crate async_trait;

pub use client::{ApiBaseUrl, HttpClient};
pub use error::Error;
