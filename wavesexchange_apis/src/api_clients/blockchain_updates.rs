use crate::{ApiResult, BaseApi, Error, GrpcClient};
use itertools::Itertools;
use std::{
    collections::HashMap,
    convert::{From, Into, TryFrom, TryInto},
    sync::Arc,
};
use waves_protobuf_schemas::waves::events::{
    blockchain_updated::{Append, Update},
    grpc::{GetBlockUpdateRequest, GetBlockUpdateResponse},
    state_update::BalanceUpdate,
    BlockchainUpdated,
};

#[derive(Clone, Debug)]
pub struct BlockchainUpdates;

impl BaseApi for BlockchainUpdates {}

impl GrpcClient<BlockchainUpdates> {
    pub async fn fetch_transactions_at_height(
        &self,
        height: u32,
    ) -> ApiResult<TransactionsAtHeight> {
        let request = tonic::Request::new(GetBlockUpdateRequest {
            height: height as i32,
        });

        self.grpc_client
            .clone()
            .get_block_update(request)
            .await
            .map_err(Arc::new)?
            .into_inner()
            .try_into()
            .map_err(|err| match err {
                ConvertError::NotFound => Error::ResponseParseError(format!(
                    "Requested block update not found at height {}",
                    height
                )),
                ConvertError::NoUpdate => {
                    Error::ResponseParseError("Expected Append Update, found None".to_string())
                }
                ConvertError::RollbackUpdate => Error::ResponseParseError(
                    "Expected Append Update, found Rollback Update".to_string(),
                ),
            })
    }
}

#[derive(Clone, Debug)]
pub struct TransactionsAtHeight {
    pub height: u32,
    pub transactions: TransactionsBalances,
}

#[derive(Clone, Debug)]
pub struct TransactionsBalances {
    pub tx_by_id: HashMap<TxId, AddressBalances>,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct TxId(pub String);

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Address(pub String);

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct AssetId(pub String);

#[derive(Clone, Default, Debug)]
pub struct AddressBalances {
    pub balances_by_address: HashMap<Address, AssetBalances>,
}

#[derive(Clone, Default, Debug)]
pub struct AssetBalances {
    pub balance_change_by_asset: HashMap<AssetId, AmountChange>,
}

#[derive(Clone, Debug)]
pub struct AmountChange {
    pub before: i64,
    pub after: i64,
}

#[derive(Clone, Copy, Debug)]
pub enum ConvertError {
    NotFound,
    NoUpdate,
    RollbackUpdate,
}

impl TryFrom<GetBlockUpdateResponse> for TransactionsAtHeight {
    type Error = ConvertError;

    fn try_from(res: GetBlockUpdateResponse) -> Result<TransactionsAtHeight, ConvertError> {
        match res.update {
            None => Err(ConvertError::NotFound),
            Some(update) => update.try_into(),
        }
    }
}

impl TryFrom<BlockchainUpdated> for TransactionsAtHeight {
    type Error = ConvertError;

    fn try_from(update: BlockchainUpdated) -> Result<TransactionsAtHeight, ConvertError> {
        let (height, update) = (update.height, update.update);
        match update {
            None => Err(ConvertError::NoUpdate),
            Some(Update::Rollback(_)) => Err(ConvertError::RollbackUpdate),
            Some(Update::Append(append)) => {
                let txs = TransactionsAtHeight {
                    height: height as u32,
                    transactions: append.into(),
                };
                Ok(txs)
            }
        }
    }
}

impl From<Append> for TransactionsBalances {
    fn from(append: Append) -> TransactionsBalances {
        let ids = append
            .transaction_ids
            .into_iter()
            .map(|id| TxId(bs58::encode(id).into_string()));
        let balances = append
            .transaction_state_updates
            .into_iter()
            .map(|st| st.balances);
        let ids_balances = ids.zip(balances);
        let tx_by_id = ids_balances
            .map(|(id, balances)| (id, balances.into()))
            .collect();
        TransactionsBalances { tx_by_id }
    }
}

impl From<Vec<BalanceUpdate>> for AddressBalances {
    fn from(balance_updates: Vec<BalanceUpdate>) -> AddressBalances {
        let res = balance_updates
            .into_iter()
            .map(|balance_update| {
                let address = Address(bs58::encode(&balance_update.address).into_string());
                let before = balance_update.amount_before;
                let after = balance_update.amount_after.as_ref().map(|amt| {
                    let asset_id = if amt.asset_id.is_empty() {
                        AssetId("WAVES".to_string())
                    } else {
                        AssetId(bs58::encode(&amt.asset_id).into_string())
                    };
                    let amount = amt.amount;
                    (asset_id, amount)
                });
                (address, before, after)
            })
            .filter_map(|(address, amount_before, after)| {
                after.map(|(asset_id, amount_after)| {
                    (address, (asset_id, amount_before, amount_after))
                })
            })
            .into_grouping_map()
            .aggregate(|acc, _, (asset_id, amount_before, amount_after)| {
                let mut balances = acc.unwrap_or_else(AssetBalances::default);
                let change = AmountChange {
                    before: amount_before,
                    after: amount_after,
                };
                balances.balance_change_by_asset.insert(asset_id, change);
                Some(balances)
            });

        AddressBalances {
            balances_by_address: res,
        }
    }
}
