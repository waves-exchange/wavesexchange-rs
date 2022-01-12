mod factory;

pub type AssetId = String;
pub type DefoSymbol = String;
pub type AnyStakeId = usize;
pub type LeveragedPairId = String;

pub use factory::Factory;

#[derive(Debug, Clone)]
pub struct Asset {
    pub id: AssetId,
    pub usd_like: bool,
    pub info: Info,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Info {
    USDN,
    Defo {
        symbol: DefoSymbol,
    },
    AnyStakeShare {
        product_id: String,
        any_stake_id: AnyStakeId,
        base_asset_id: AssetId,
    },
    Leveraged {
        pair_id: LeveragedPairId,
        bull_bear: BullBear,
    },
    LiquidityPool(LiquidityPoolAssetInfo),
    Common,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BullBear {
    Bull,
    Bear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidityPoolAssetInfo {
    pub address: String,
    pub amount_asset_id: String,
    pub price_asset_id: String,
    pub price_decimals: i64,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl AsRef<str> for Asset {
    fn as_ref(&self) -> &str {
        self.id.as_ref()
    }
}

impl From<Asset> for String {
    fn from(a: Asset) -> Self {
        a.id
    }
}
