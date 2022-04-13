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

pub use assets::AssetsService;
pub use blockchain_updates::BlockchainUpdates;
pub use data_service::DataService;
pub use identity::Identity;
pub use interest_rates::InterestService;
pub use levex::Levex;
pub use liquidity_pools::LiquidityPools;
pub use matcher::Matcher;
pub use node::Node;
pub use rate_aggregates::RateAggregates;
pub use rates::RatesService;
pub use state::StateService;
pub use transfers::Transfers;

use std::fmt::Debug;

pub trait BaseApi: Sync + Clone + Debug {
    const MAINNET_URL: &'static str;
    const TESTNET_URL: &'static str;
}
