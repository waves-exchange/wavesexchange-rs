pub mod assets_service;
pub mod blockchain_updates;
pub mod data_service;
pub mod levex;
pub mod node;
pub mod state_service;

pub use assets_service::AssetsSvcApi;
pub use blockchain_updates::BlockchainUpdApi;
pub use data_service::DataSvcApi;
pub use levex::LevexApi;
pub use node::NodeApi;
pub use state_service::StateSvcApi;

pub trait BaseApi: Clone {}
