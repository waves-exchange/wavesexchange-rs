pub mod assets_service;
pub mod blockchain_updates;
pub mod data_service;
pub mod interest_service;
pub mod levex;
pub mod liquidity_pools;
pub mod matcher;
pub mod node;
pub mod rate_aggregates;
pub mod rates_service;
pub mod state_service;
pub mod transfers;

pub use assets_service::AssetsSvcApi;
pub use blockchain_updates::BlockchainUpdApi;
pub use data_service::DataSvcApi;
pub use interest_service::InterestSvcApi;
pub use levex::LevexApi;
pub use liquidity_pools::LiquidityPoolsApi;
pub use matcher::MatcherApi;
pub use node::NodeApi;
pub use rate_aggregates::RateAggregatesApi;
pub use rates_service::RatesSvcApi;
pub use state_service::StateSvcApi;
pub use transfers::TransfersApi;

use std::fmt::Debug;

pub trait BaseApi: Sync + Clone + Debug {}
