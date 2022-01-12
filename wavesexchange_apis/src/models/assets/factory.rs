use super::{Asset, AssetId, Info};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Factory {
    pub usdn_asset_id: String,
    dictionary: HashMap<AssetId, Asset>,
}

impl Factory {
    pub fn new(
        usdn_asset_id: impl Into<String>,
        known_assets: impl IntoIterator<Item = Asset>,
    ) -> Self {
        let mut dict = HashMap::new();

        let usdn_asset_id = usdn_asset_id.into();
        dict.insert(
            usdn_asset_id.clone(),
            Asset {
                id: usdn_asset_id.clone(),
                usd_like: true,
                info: Info::USDN,
            },
        );

        dict.extend(known_assets.into_iter().map(|a| (a.id.to_owned(), a)));

        Self {
            usdn_asset_id,
            dictionary: dict,
        }
    }

    pub fn new_asset(&self, asset_id: impl AsRef<str>) -> Asset {
        self.dictionary
            .get(asset_id.as_ref())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Asset {
                id: asset_id.as_ref().to_owned(),
                usd_like: false,
                info: Info::Common,
            })
    }

    pub fn usdn(&self) -> Asset {
        Asset {
            id: self.usdn_asset_id.clone(),
            usd_like: true,
            info: Info::USDN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::assets::Info::*;
    use crate::tests::blockchains::TESTNET;

    // test ids
    const USDN_ID: &str = "usdn";
    const EURN_ID: &str = "eurn";
    const EURN_SYMBOL: &str = "EUR";
    const USDT_ID: &str = "usdt";
    const USDTLP_ID: &str = "usdtlp";
    const AUSDTLPA_ID: &str = "ausdtlpa";

    #[test]
    fn factory() {
        let any_stake_product_id = TESTNET::products[0].id.to_owned();
        let algo_product_id = TESTNET::products[1].id.to_owned();

        let factory = Factory::new(
            USDN_ID,
            vec![
                Asset {
                    id: EURN_ID.to_owned(),
                    usd_like: false,
                    info: Defo {
                        symbol: EURN_SYMBOL.to_owned(),
                    },
                },
                Asset {
                    id: USDTLP_ID.to_owned(),
                    usd_like: false,
                    info: AnyStakeShare {
                        product_id: any_stake_product_id.clone(),
                        any_stake_id: 0,
                        base_asset_id: USDT_ID.to_owned(),
                    },
                },
                Asset {
                    id: AUSDTLPA_ID.to_owned(),
                    usd_like: false,
                    info: AnyStakeShare {
                        product_id: algo_product_id.clone(),
                        any_stake_id: 0,
                        base_asset_id: USDT_ID.to_owned(),
                    },
                },
            ],
        );

        let usdn = factory.new_asset(USDN_ID);
        assert_eq!(usdn.id, USDN_ID);
        assert_eq!(usdn.info, Info::USDN);

        let usdlp = factory.new_asset(USDTLP_ID);
        assert_eq!(usdlp.id, USDTLP_ID);
        match usdlp.info {
            AnyStakeShare {
                product_id,
                any_stake_id,
                base_asset_id,
            } => {
                assert_eq!(product_id, any_stake_product_id);
                assert_eq!(any_stake_id, 0);
                assert_eq!(base_asset_id, USDT_ID);
            }
            _ => panic!(),
        };

        let ausdtlpm = factory.new_asset(AUSDTLPA_ID);
        assert_eq!(ausdtlpm.id, AUSDTLPA_ID);
        match ausdtlpm.info {
            AnyStakeShare {
                product_id,
                any_stake_id,
                base_asset_id,
            } => {
                assert_eq!(product_id, algo_product_id);
                assert_eq!(any_stake_id, 0);
                assert_eq!(base_asset_id, USDT_ID);
            }
            _ => panic!(),
        };
    }
}
