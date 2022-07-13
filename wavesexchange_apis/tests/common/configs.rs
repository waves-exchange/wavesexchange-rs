#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use once_cell::sync::Lazy;

pub struct Product {
    pub id: String,
    pub contract_address: String,
    pub asset_pairs: Vec<(String, String)>,
}

pub mod MAINNET {
    use super::*;

    // addresses
    pub const matcher: &str = "3PEjHv3JGjcWNpYEEkif2w8NXV4kbhnoGgu";
    pub const defo_control_contract: &str = "3P8qJyxUqizCWWtEn2zsLZVPzZAjdNGppB1";
    pub const defo_factory_contract: &str = "3PQEjFmdcjd6wf1TrpkHSuDAk3zbfLSeikb";
    pub const lp_factory_contract: &str = "3P7YV1GYyfcAZMy6dmJdJv6zrDp7nZsNexT";
    pub const lp_rest_contract: &str = "3P8MoPnsaurofk1VyhsdAFkeQ6ijpJYXCpW";

    // upstream
    pub const data_service_url: &str = "https://waves.exchange/api/v1/forward/data_service/v0";
    pub const node_url: &str = "https://nodes.waves.exchange";
    pub const matcher_api_url: &str = "https://matcher.waves.exchange/matcher/settings/rates";
    pub const state_service_url: &str = "https://waves.exchange/api/v1/state";
    pub const assets_service_url: &str = "https://waves.exchange/api/v1/assets";
    pub const blockchain_updates_url: &str = "https://blockchain-updates.waves.exchange";

    // assets
    pub const usdn_asset_id: &str = "DG2xFkPdDwKUoBkzGAhQtLpSGzfXLiCYPEzeKH2Ad24p";
    pub static usd_like_assets: Lazy<Vec<&str>> = Lazy::new(|| {
        vec![
            "34N9YcEETLWn93qYQ64EsP1x89tSruJU44RrEMSXXEPJ",
            "6XtHjpXbs9RRJP2Sr9GUyVqzACcby9TkThHXnjVC5CDJ",
            "AEhWyMGY3twQdPQSCqVSdVqxcgzyEn129ipzvbqtTcyv",
        ]
    });

    // any_stake
    pub static products: Lazy<Vec<Product>> = Lazy::new(|| {
        vec![Product {
            id: "any_stake".to_owned(),
            contract_address: "3P6SFR9ZZwKHZw5mMDZxpXHEhg1CXjBb51y".to_owned(),
            asset_pairs: vec![
                (
                    "9AT2kEi8C4AYxV1qKxtQTVpD5i54jCPvaNNRP6VzRtYZ".to_owned(),
                    "34N9YcEETLWn93qYQ64EsP1x89tSruJU44RrEMSXXEPJ".to_owned(),
                ),
                (
                    "CrjhbC9gRezwvBZ1XwPQqRwx4BWzoyMHGFNUVdn22ep6".to_owned(),
                    "6XtHjpXbs9RRJP2Sr9GUyVqzACcby9TkThHXnjVC5CDJ".to_owned(),
                ),
                (
                    "DazN41oAedqwGZ8aabf4nJQwJNZhsEgPH3YQWDtPsdeV".to_owned(),
                    "8LQW8f7P5d5PZM7GtZEBgaqRPGSzS3DfPuiXrURJ4AJS".to_owned(),
                ),
                (
                    "ELzXTgPa6GGYyLtitn2oWDWQ9joyTFEueNtF4kxg95dM".to_owned(),
                    "474jTeYx2r2Va35794tCScAXWJG9hU2HcgxzMowaZUnu".to_owned(),
                ),
            ],
        }]
    });
}

pub mod TESTNET {
    use super::*;

    // addresses
    pub const matcher: &str = "3N8aZG6ZDfnh8YxS6aNcteobN8eXTWHaBBd";
    pub const defo_control_contract: &str = "3MyAeWKH4gAS6iDvTRo42Rz3WCoMEi63WMC";
    pub const defo_factory_contract: &str = "3N3UHNYQ8xwe2WWLKgXX628JyUtF6kYK4wS";
    pub const lp_factory_contract: &str = "3MxueHaGvWmdk5crtn9HwtkSAAZScTWoHaC";
    pub const lp_rest_contract: &str = "3MsNhK6uve98J6DeqbuwGFBRh9GoHPGUFgp";

    // upstream
    pub const data_service_url: &str =
        "https://testnet.waves.exchange/api/v1/forward/data_service/v0";
    pub const node_url: &str = "https://nodes-testnet.wavesnodes.com";
    pub const matcher_api_url: &str =
        "https://matcher-testnet.waves.exchange/matcher/settings/rates";
    pub const state_service_url: &str = "https://testnet.waves.exchange/api/v1/state";
    pub const assets_service_url: &str = "https://testnet.waves.exchange/api/v1/assets";
    pub const blockchain_updates_url: &str = "https://blockchain-updates-testnet.waves.exchange";

    // assets
    pub const usdn_asset_id: &str = "25FEqEjRkqK6yCkiT7Lz6SAYz7gUFCtxfCChnrVFD5AT";
    // todo insert testnet USD* assets
    pub const usd_like_assets: Vec<&str> = vec![];

    // any_stake
    pub static products: Lazy<Vec<Product>> = Lazy::new(|| {
        vec![
            Product {
                id: "any_stake".to_owned(),
                contract_address: "3Mzt645zA6u2QG6jRPoo6H6CK89kVggFgNi".to_owned(),
                asset_pairs: vec![(
                    "4stNN53V8P3GpsuGrqH4cqvGg83XELR9kh5Y2ayxZDDu".to_owned(),
                    "5Sh9KghfkZyhjwuodovDhB6PghDUGBHiAPZ4MkrPgKtX".to_owned(),
                )],
            },
            Product {
                id: "algo_trading_moderate".to_owned(),
                contract_address: "3NC9wWawxuFG6a3sZdfckGwoMeVhLFjZFwH".to_owned(),
                asset_pairs: vec![(
                    "CNMLmtfBvX6R9MNmaMvRxJR2Kgt4MHUoWcPuqYDVdimu".to_owned(),
                    "5Sh9KghfkZyhjwuodovDhB6PghDUGBHiAPZ4MkrPgKtX".to_owned(),
                )],
            },
        ]
    });
}
