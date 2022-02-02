pub mod assets;
pub mod blockchain_updates;
pub mod data_service;
pub mod identity;
pub mod interest_rates;
pub mod levex;
pub mod liquidity_pools;
pub mod matcher;
pub mod node;
pub mod rate_aggregates;
pub mod rates;
pub mod state;
pub mod transfers;

pub use assets::AssetsSvcApi;
pub use blockchain_updates::BlockchainUpdApi;
pub use data_service::DataSvcApi;
pub use identity::IdentityApi;
pub use interest_rates::InterestSvcApi;
pub use levex::LevexApi;
pub use liquidity_pools::LiquidityPoolsApi;
pub use matcher::MatcherApi;
pub use node::NodeApi;
pub use rate_aggregates::RateAggregatesApi;
pub use rates::RatesSvcApi;
pub use state::StateSvcApi;
pub use transfers::TransfersApi;

use std::fmt::Debug;

pub trait BaseApi: Sync + Clone + Debug {}
