//! Subscription topic: an URI which can be parsed
//! into a machine-readable data struct describing client's subscription.

use std::sync::Arc;
use url::Url;

pub use parse_and_format::parse::TopicParseError;

/// A cheaply cloneable (`Arc` inside) subscription topic struct.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Topic {
    topic_url: Arc<Url>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TopicKind {
    Config,
    State,
    TestResource,
    BlockchainHeight,
    Transaction,
    LeasingBalance,
    Pairs,
}

/// A parsed Topic representation
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TopicData {
    Config(ConfigResource),
    State(State),
    TestResource(TestResource),
    BlockchainHeight(BlockchainHeight),
    Transaction(Transaction),
    LeasingBalance(LeasingBalance),
    Pairs(Vec<ExchangePairs>),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigResource {
    pub file: ConfigFile,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigFile {
    pub path: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum State {
    Single(StateSingle),
    MultiPatterns(StateMultiPatterns),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct StateSingle {
    pub address: String,
    pub key: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct StateMultiPatterns {
    pub addresses: Vec<String>,
    pub key_patterns: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TestResource {
    pub path: String,
    pub query: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BlockchainHeight;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Transaction {
    ByAddress(TransactionByAddress),
    Exchange(TransactionExchange),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TransactionByAddress {
    pub tx_type: TransactionType,
    pub address: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TransactionType {
    All,
    Genesis,
    Payment,
    Issue,
    Transfer,
    Reissue,
    Burn,
    Exchange,
    Lease,
    LeaseCancel,
    Alias,
    MassTransfer,
    Data,
    SetScript,
    Sponsorship,
    SetAssetScript,
    InvokeScript,
    UpdateAssetInfo,
    InvokeExpression,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TransactionExchange {
    pub amount_asset: String,
    pub price_asset: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LeasingBalance {
    pub address: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ExchangePairs {
    pub amount_asset: String,
    pub price_asset: String,
}

mod parse_and_format {
    pub(super) mod parse {
        use std::{borrow::Cow, sync::Arc};
        use thiserror::Error;
        use url::Url;

        use crate::ExchangePairs;

        use super::super::{
            BlockchainHeight, ConfigFile, ConfigResource, LeasingBalance, State, StateSingle,
            TestResource, Topic, TopicData, TopicKind, Transaction, TransactionByAddress,
            TransactionExchange, TransactionType,
        };
        use super::{maybe_string::MaybeString, serde_state, url_escape};

        #[derive(Debug, PartialEq, Eq, Error)]
        pub enum TopicParseError {
            #[error("Topic URI cannot be parsed: {0}")]
            UrlParseError(#[from] url::ParseError),

            #[error("Malformed topic")]
            MalformedTopic,

            #[error("Invalid topic kind: {0}")]
            InvalidTopicKind(MaybeString),

            #[error("Invalid 'config' topic")]
            InvalidConfigTopic,

            #[error("Invalid 'state' topic")]
            InvalidStateTopic,

            #[error("Invalid 'test resource' topic")]
            InvalidTestResourceTopic,

            #[error("Invalid 'blockchain height' topic")]
            InvalidBlockchainHeightTopic,

            #[error("Invalid 'transaction' topic")]
            InvalidTransactionTopic,

            #[error("Invalid 'leasing balance' topic")]
            InvalidLeasingBalanceTopic,

            #[error("Invalid transaction type: {0}")]
            InvalidTransactionType(MaybeString),

            #[error("Invalid exchange pairs data")]
            InvalidExchangePairs,
        }

        impl Topic {
            pub fn parse_str(topic_uri: &str) -> Result<Self, TopicParseError> {
                let mut url = Url::parse(topic_uri)?;
                Self::validate_and_canonicalize_topic_url(&mut url)?;

                Ok(Topic {
                    topic_url: Arc::new(url),
                })
            }

            fn validate_and_canonicalize_topic_url(url: &mut Url) -> Result<(), TopicParseError> {
                if url.scheme() != "topic"
                    || url.cannot_be_a_base()
                    || url.username() != ""
                    || url.password().is_some()
                    || url.fragment().is_some()
                    || url.port().is_some()
                {
                    return Err(TopicParseError::MalformedTopic);
                }

                let topic_kind_str = url
                    .host_str()
                    .ok_or(TopicParseError::InvalidTopicKind(MaybeString(None)))?;

                let topic_kind = TopicKind::parse(topic_kind_str).ok_or_else(|| {
                    TopicParseError::InvalidTopicKind(MaybeString(Some(topic_kind_str.to_owned())))
                })?;

                fn is_empty(s: Option<impl AsRef<str>>) -> bool {
                    match s {
                        None => true,
                        Some(s) => s.as_ref().is_empty(),
                    }
                }

                match topic_kind {
                    TopicKind::Config => {
                        let config_file_path = url.path();
                        if config_file_path.is_empty() || url.query().is_some() {
                            return Err(TopicParseError::InvalidConfigTopic);
                        }
                    }
                    TopicKind::State => {
                        let is_single = url.query().is_none();
                        if is_single {
                            // unwrap() is safe here because we've already checked for `cannot_be_a_base()`
                            let mut path_segments = url.path_segments().unwrap();
                            let address = path_segments.next();
                            let key = path_segments.next();
                            if is_empty(address) || is_empty(key) || path_segments.next().is_some()
                            {
                                return Err(TopicParseError::InvalidStateTopic);
                            }
                            // Canonicalize
                            url.set_path(&format!(
                                "{}/{}",
                                url_escape::encode(
                                    address.map(url_escape::decode).unwrap().as_ref()
                                ),
                                url_escape::encode(key.map(url_escape::decode).unwrap().as_ref())
                            ));
                        } else {
                            let is_ok = url.query_pairs().all(|(k, v)| {
                                let key = url_escape::decode(&*k);
                                let key_ok = key.starts_with("address__in[")
                                    || key.starts_with("key__match_any[");
                                let value_ok = !v.is_empty();
                                key_ok && value_ok
                            });
                            if !is_ok {
                                return Err(TopicParseError::InvalidStateTopic);
                            }
                            // Canonicalize
                            let query = url.query().unwrap(); // unwrap is safe here
                            let st = serde_state::state_query_decode(query)
                                .map_err(|()| TopicParseError::InvalidStateTopic)?;
                            let query = serde_state::state_query_encode(&st)
                                .map_err(|()| TopicParseError::InvalidStateTopic)?;
                            url.set_query(Some(&query));
                        }
                    }
                    TopicKind::TestResource => {
                        let is_ok = !url.path().is_empty() || !is_empty(url.query());
                        if !is_ok {
                            return Err(TopicParseError::InvalidTestResourceTopic);
                        }
                    }
                    TopicKind::BlockchainHeight => {
                        let is_ok = url.path().is_empty() && is_empty(url.query());
                        if !is_ok {
                            return Err(TopicParseError::InvalidBlockchainHeightTopic);
                        }
                    }
                    TopicKind::Transaction => {
                        let tx_type = query_get(url, "type")
                            .map(|s| {
                                TransactionType::parse(&*s).ok_or_else(|| {
                                    TopicParseError::InvalidTransactionType(
                                        MaybeString::from_emptyable_str(&*s),
                                    )
                                })
                            })
                            .transpose()?;

                        let is_exchange = if matches!(tx_type, Some(TransactionType::Exchange)) {
                            let price_asset = query_get(url, "price_asset");
                            let amount_asset = query_get(url, "amount_asset");
                            let has_price_asset = !is_empty(price_asset);
                            let has_amount_asset = !is_empty(amount_asset);
                            if has_price_asset != has_amount_asset {
                                return Err(TopicParseError::InvalidTransactionTopic);
                            }
                            has_price_asset && has_amount_asset
                        } else {
                            false
                        };

                        let address = query_get(url, "address");

                        let is_ok = if is_exchange {
                            is_empty(address)
                        } else {
                            !is_empty(address)
                        };

                        if !is_ok {
                            return Err(TopicParseError::InvalidTransactionTopic);
                        }
                    }
                    TopicKind::LeasingBalance => {
                        // unwrap() is safe here because we've already checked for `cannot_be_a_base()`
                        let mut path_segments = url.path_segments().unwrap();
                        let address = path_segments.next();
                        if is_empty(address)
                            || path_segments.next().is_some()
                            || !is_empty(url.query())
                        {
                            return Err(TopicParseError::InvalidLeasingBalanceTopic);
                        }
                    }
                    TopicKind::Pairs => {
                        Topic::extract_exchange_pairs_from_query(&url)?;
                    }
                }

                Ok(())
            }

            pub fn extract_exchange_pairs_from_query<'a>(
                url: &'a Url,
            ) -> Result<Vec<ExchangePairs>, TopicParseError> {
                let mut ret = vec![];

                match url.path_segments() {
                    Some(mut parts) => {
                        // topic://pairs/<amount_asset_id>/<price_asset_id>
                        let amount_asset = parts.next();
                        let price_asset = parts.next();

                        if amount_asset.is_some() && price_asset.is_some() {
                            ret.push({
                                ExchangePairs {
                                    amount_asset: amount_asset.unwrap().into(),
                                    price_asset: price_asset.unwrap().into(),
                                }
                            });
                            return Ok(ret);
                        }

                        // topic://pairs?pairs[]=amount_asset_id/price_asset_id&pairs[]=amount_asset_id1/price_asset_id1

                        let pairs = query_get_vec(url, "pairs");
                        match pairs {
                            None => {
                                return Err(TopicParseError::InvalidExchangePairs);
                            }
                            Some(pairs) => {
                                for p in pairs {
                                    if !p.contains("/") {
                                        return Err(TopicParseError::InvalidExchangePairs);
                                    }
                                    let pair: Vec<&str> = p.split("/").collect();
                                    let mut iter_pair = pair.iter();
                                    let p = ExchangePairs {
                                        amount_asset: (*iter_pair.next().unwrap()).into(),
                                        price_asset: (*iter_pair.next().unwrap()).into(),
                                    };

                                    ret.push(p);
                                }
                            }
                        }
                    }
                    None => {}
                }

                Ok(ret)
            }
        }

        impl TopicData {
            pub(in super::super) fn parse(topic: &Topic) -> Self {
                let url = topic.topic_url.as_ref();

                // This is checked by `validate()` during parse stage, so `expect()` is safe
                let topic_kind_str = url.host_str().expect("host_str");

                // Same safety guarantee
                let topic_kind = TopicKind::parse(topic_kind_str).expect("topic_kind");

                // URL is checked by `validate()` during parse stage, so all `expect()` calls are safe
                match topic_kind {
                    TopicKind::Config => {
                        let config_file_path = url.path().to_owned();
                        TopicData::Config(ConfigResource {
                            file: ConfigFile {
                                path: config_file_path,
                            },
                        })
                    }
                    TopicKind::State => TopicData::State({
                        let is_single = url.query().is_none();
                        if is_single {
                            State::Single({
                                let mut path = url.path_segments().expect("path_segments");
                                let address =
                                    url_escape::decode(path.next().expect("path[0]")).into_owned();
                                let key =
                                    url_escape::decode(path.next().expect("path[1]")).into_owned();
                                assert!(path.next().is_none(), "path.length");
                                StateSingle { address, key }
                            })
                        } else {
                            State::MultiPatterns({
                                let query = url.query().expect("query");
                                serde_state::state_query_decode(query).expect("state_query_decode")
                            })
                        }
                    }),
                    TopicKind::TestResource => TopicData::TestResource({
                        TestResource {
                            path: url.path().to_owned(),
                            query: url.query().map(|q| q.to_owned()),
                        }
                    }),
                    TopicKind::BlockchainHeight => TopicData::BlockchainHeight(BlockchainHeight),
                    TopicKind::Transaction => TopicData::Transaction({
                        let tx_type = query_get(url, "type")
                            .map(|s| TransactionType::parse(&*s).expect("tx_type"));

                        let tx = if matches!(tx_type, Some(TransactionType::Exchange)) {
                            let price_asset = query_get(url, "price_asset");
                            let amount_asset = query_get(url, "amount_asset");
                            if let (Some(price_asset), Some(amount_asset)) =
                                (price_asset, amount_asset)
                            {
                                Some(TransactionExchange {
                                    amount_asset: amount_asset.to_string(),
                                    price_asset: price_asset.to_string(),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(tx) = tx {
                            Transaction::Exchange(tx)
                        } else {
                            let address = query_get(url, "address").expect("address");
                            let tx_type = tx_type.unwrap_or(TransactionType::All);
                            Transaction::ByAddress(TransactionByAddress {
                                tx_type,
                                address: address.to_string(),
                            })
                        }
                    }),
                    TopicKind::LeasingBalance => TopicData::LeasingBalance({
                        let mut path_segments = url.path_segments().expect("path_segments");
                        let address = path_segments.next().expect("path[0]");
                        assert!(path_segments.next().is_none(), "path.length");
                        LeasingBalance {
                            address: address.to_owned(),
                        }
                    }),
                    TopicKind::Pairs => TopicData::Pairs(
                        Topic::extract_exchange_pairs_from_query(&url).expect("pairs"),
                    ),
                }
            }
        }

        fn query_get<'a>(url: &'a Url, key: &str) -> Option<Cow<'a, str>> {
            url.query_pairs().find_map(|(k, v)| {
                if k == key && !v.is_empty() {
                    Some(v)
                } else {
                    None
                }
            })
        }

        fn query_get_vec<'a>(url: &'a Url, key: &str) -> Option<Vec<Cow<'a, str>>> {
            let mut vals: Vec<Cow<_>> = vec![];

            for i in url.query_pairs().into_iter() {
                let k =
                    i.0.replace("%5B", "")
                        .replace("%5D", "")
                        .replace("[", "")
                        .replace("]", "");
                if k == key && !(i.1).is_empty() {
                    vals.push(i.1);
                }
            }

            if vals.is_empty() {
                return None;
            }

            Some(vals)
        }

        impl TopicKind {
            pub(in super::super) fn parse(s: &str) -> Option<Self> {
                match s {
                    "config" => Some(TopicKind::Config),
                    "state" => Some(TopicKind::State),
                    "test_resource" => Some(TopicKind::TestResource),
                    "blockchain_height" => Some(TopicKind::BlockchainHeight),
                    "transactions" => Some(TopicKind::Transaction),
                    "leasing_balance" => Some(TopicKind::LeasingBalance),
                    "pairs" => Some(TopicKind::Pairs),
                    _ => None,
                }
            }
        }

        impl TransactionType {
            fn parse(s: &str) -> Option<Self> {
                let transaction_type = match s {
                    "all" => TransactionType::All,
                    "genesis" => TransactionType::Genesis,
                    "payment" => TransactionType::Payment,
                    "issue" => TransactionType::Issue,
                    "transfer" => TransactionType::Transfer,
                    "reissue" => TransactionType::Reissue,
                    "burn" => TransactionType::Burn,
                    "exchange" => TransactionType::Exchange,
                    "lease" => TransactionType::Lease,
                    "lease_cancel" => TransactionType::LeaseCancel,
                    "alias" => TransactionType::Alias,
                    "mass_transfer" => TransactionType::MassTransfer,
                    "data" => TransactionType::Data,
                    "set_script" => TransactionType::SetScript,
                    "sponsorship" => TransactionType::Sponsorship,
                    "set_asset_script" => TransactionType::SetAssetScript,
                    "invoke_script" => TransactionType::InvokeScript,
                    "update_asset_info" => TransactionType::UpdateAssetInfo,
                    "invoke_expression" => TransactionType::InvokeExpression,
                    _ => return None,
                };
                Some(transaction_type)
            }
        }

        #[test]
        fn topic_kind_test() -> anyhow::Result<()> {
            let topic_urls = [
                ("topic://config/some/path", TopicKind::Config),
                ("topic://state/address/key", TopicKind::State),
                ("topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern2", TopicKind::State),
                ("topic://test_resource/some/path?and_query=true", TopicKind::TestResource),
                ("topic://blockchain_height", TopicKind::BlockchainHeight),
                ("topic://transactions?type=all&address=some_address", TopicKind::Transaction),
                ("topic://transactions?type=exchange&amount_asset=foo&price_asset=bar", TopicKind::Transaction),
                ("topic://leasing_balance/some_address", TopicKind::LeasingBalance),
                ("topic://pairs/amount_asset/price_asset", TopicKind::Pairs),
                ("topic://pairs/?pair[]=amount_asset/price_asset&pairs[]=amount_asset1/price_asset1", TopicKind::Pairs),
            ];
            for &(topic_url, expected_kind) in topic_urls.iter() {
                let url = Url::parse(topic_url)?;
                let kind_str = url
                    .host_str()
                    .ok_or_else(|| anyhow::anyhow!("bad test case: {}", topic_url))?;
                let kind = TopicKind::parse(kind_str)
                    .ok_or_else(|| anyhow::anyhow!("bad test case: {}", kind_str))?;
                assert_eq!(kind, expected_kind);
                drop(url);

                let topic = Topic::parse_str(topic_url)?;
                let kind = topic.kind();
                assert_eq!(kind, expected_kind);
            }
            Ok(())
        }

        #[test]
        fn topic_state_test() -> anyhow::Result<()> {
            let topic_data = Topic::parse_str("topic://state/some_address/some_key")?.data();
            let state = topic_data
                .as_state()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            assert!(matches!(state, State::Single(_)));
            if let State::Single(ref state) = state {
                assert_eq!(state.address, "some_address".to_string());
                assert_eq!(state.key, "some_key".to_string());
            }

            let error = Topic::parse_str("topic://state/some_address/some_key/invalid_part");
            assert_eq!(error.unwrap_err(), TopicParseError::InvalidStateTopic);

            // URL with plain (not percent-encoded) character '*' should work
            let topic_data = Topic::parse_str("topic://state?address__in[]=addr1&address__in[]=addr2&key__match_any[]=pattern1&key__match_any[]=pattern*2")?.data();
            let state = topic_data
                .as_state()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            assert!(matches!(state, State::MultiPatterns(_)));
            if let State::MultiPatterns(ref state) = state {
                assert_eq!(state.addresses, vec!["addr1", "addr2"]);
                assert_eq!(state.key_patterns, vec!["pattern1", "pattern*2"]);
            }
            assert_eq!(
                "topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern*2".to_string(),
                topic_data.as_uri_string(),
            );

            // URL with properly percent-encoded chars should also work
            let topic_data = Topic::parse_str("topic://state?address__in[]=addr1&address__in[]=addr2&key__match_any[]=pattern1&key__match_any[]=pattern%2A2")?.data();
            let state = topic_data
                .as_state()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            assert!(matches!(state, State::MultiPatterns(_)));
            if let State::MultiPatterns(ref state) = state {
                assert_eq!(state.addresses, vec!["addr1", "addr2"]);
                assert_eq!(state.key_patterns, vec!["pattern1", "pattern*2"]);
            }
            assert_eq!(
                "topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern*2".to_string(),
                topic_data.as_uri_string(),
            );

            Ok(())
        }

        #[test]
        fn transaction_topic_test() -> anyhow::Result<()> {
            let topic_data =
                Topic::parse_str("topic://transactions?type=all&address=some_address")?.data();
            let tx = topic_data
                .as_transaction()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            if let Transaction::ByAddress(transaction) = tx.clone() {
                assert_eq!(transaction.tx_type.to_string(), "all".to_string());
                assert_eq!(transaction.address, "some_address".to_string());
                assert_eq!(
                    "topic://transactions?type=all&address=some_address".to_string(),
                    TopicData::Transaction(Transaction::ByAddress(transaction)).as_uri_string(),
                );
            } else {
                panic!("wrong transaction")
            }

            let topic_data =
                Topic::parse_str("topic://transactions?type=issue&address=some_other_address")?
                    .data();
            let tx = topic_data
                .as_transaction()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            if let Transaction::ByAddress(transaction) = tx.clone() {
                assert_eq!(transaction.tx_type.to_string(), "issue".to_string());
                assert_eq!(transaction.address, "some_other_address".to_string());
                assert_eq!(
                    "topic://transactions?type=issue&address=some_other_address".to_string(),
                    TopicData::Transaction(Transaction::ByAddress(transaction)).as_uri_string()
                );
            }

            let error = Topic::parse_str("topic://transactions");
            assert!(error.is_err());
            assert_eq!(error.unwrap_err(), TopicParseError::InvalidTransactionTopic);

            let topic_data = Topic::parse_str(
                "topic://transactions?type=exchange&amount_asset=asd&price_asset=qwe",
            )?
            .data();
            let tx = topic_data
                .as_transaction()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            if let Transaction::Exchange(transaction) = tx.clone() {
                assert_eq!(transaction.amount_asset, "asd".to_string());
                assert_eq!(transaction.price_asset, "qwe".to_string());
                assert_eq!(
                    "topic://transactions?type=exchange&amount_asset=asd&price_asset=qwe"
                        .to_string(),
                    TopicData::Transaction(Transaction::Exchange(transaction)).as_uri_string()
                );
            } else {
                panic!("wrong exchange transaction")
            }

            let error = Topic::parse_str(
                "topic://transactions?type=exchange&amount_asset=asd&price_asset=",
            );
            assert!(error.is_err());

            Ok(())
        }

        #[test]
        fn leasing_balance_test() -> anyhow::Result<()> {
            let topic_data = Topic::parse_str("topic://leasing_balance/some_address")?.data();
            let leasing_balance = topic_data
                .as_leasing_balance()
                .ok_or(anyhow::anyhow!("bad test case"))?;
            assert_eq!(leasing_balance.address, "some_address".to_string());

            let error = Topic::parse_str("topic://leasing_balance/some_address/invalid_part");
            assert_eq!(
                error.unwrap_err(),
                TopicParseError::InvalidLeasingBalanceTopic,
            );

            Ok(())
        }

        #[test]
        fn pairs_one_test() -> anyhow::Result<()> {
            let topic_data = Topic::parse_str("topic://pairs/amount_asset/price_asset")?.data();
            let pairs = topic_data
                .as_pairs()
                .ok_or(anyhow::anyhow!("bad test case"))?;

            assert_eq!(pairs[0].amount_asset, "amount_asset");
            assert_eq!(pairs[0].price_asset, "price_asset");

            Ok(())
        }

        #[test]
        fn pairs_one_uri_only_test() -> anyhow::Result<()> {
            let topic_data =
                Topic::parse_str("topic://pairs/amount_asset/price_asset?pairs[]=skip/skip")?
                    .data();
            let pairs = topic_data
                .as_pairs()
                .ok_or(anyhow::anyhow!("bad test case"))?;

            assert_eq!(pairs[0].amount_asset, "amount_asset");
            assert_eq!(pairs[0].price_asset, "price_asset");

            assert_eq!(pairs.len(), 1);

            Ok(())
        }

        #[test]
        fn pairs_many_test() -> anyhow::Result<()> {
            let topic_data = Topic::parse_str("topic://pairs/?pairs[]=amount_asset/price_asset&pairs[]=amount_asset1/price_asset1")?.data();
            let pairs = topic_data
                .as_pairs()
                .ok_or(anyhow::anyhow!("bad test case"))?;

            assert_eq!(pairs[0].amount_asset, "amount_asset");
            assert_eq!(pairs[0].price_asset, "price_asset");

            assert_eq!(pairs[1].amount_asset, "amount_asset1");
            assert_eq!(pairs[1].price_asset, "price_asset1");

            Ok(())
        }

        #[test]
        fn pairs_one_error_test() -> anyhow::Result<()> {
            let topic_data = Topic::parse_str("topic://pairs/amount_asset");

            assert!(topic_data.is_err());

            Ok(())
        }

        #[test]
        fn pairs_many_error_test() -> anyhow::Result<()> {
            let topic_data =
                Topic::parse_str("?pairs[]=amount_asset/price_asset&pairs[]=amount_asset1");

            assert!(topic_data.is_err());

            Ok(())
        }
    }

    mod format {
        use crate::State;
        use std::fmt;

        use super::super::{ConfigResource, Topic, TopicData, Transaction, TransactionType};
        use super::{serde_state, url_escape};

        impl fmt::Debug for Topic {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Topic('{}')", self.topic_url.as_str())
            }
        }

        impl fmt::Display for Topic {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.topic_url.as_str())
            }
        }

        impl fmt::Display for TransactionType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let s = match self {
                    Self::All => "all",
                    Self::Genesis => "genesis",
                    Self::Payment => "payment",
                    Self::Issue => "issue",
                    Self::Transfer => "transfer",
                    Self::Reissue => "reissue",
                    Self::Burn => "burn",
                    Self::Exchange => "exchange",
                    Self::Lease => "lease",
                    Self::LeaseCancel => "lease_cancel",
                    Self::Alias => "alias",
                    Self::MassTransfer => "mass_transfer",
                    Self::Data => "data",
                    Self::SetScript => "set_script",
                    Self::Sponsorship => "sponsorship",
                    Self::SetAssetScript => "set_asset_script",
                    Self::InvokeScript => "invoke_script",
                    Self::UpdateAssetInfo => "update_asset_info",
                    Self::InvokeExpression => "invoke_expression",
                };
                write!(f, "{}", s)
            }
        }

        impl TopicData {
            pub fn as_uri_string(&self) -> String {
                let mut result = "topic://".to_string();
                match self {
                    TopicData::Config(ConfigResource { file }) => {
                        result.push_str("config");
                        result.push_str(file.path.as_str());
                    }
                    TopicData::State(State::Single(state)) => {
                        let address = url_escape::encode(&state.address);
                        let key = url_escape::encode(&state.key);
                        result.push_str(&format!("state/{}/{}", address, key));
                    }
                    TopicData::State(State::MultiPatterns(state)) => {
                        result.push_str("state?");
                        result
                            .push_str(&serde_state::state_query_encode(state).expect("urlencode"));
                    }
                    TopicData::TestResource(test_res) => {
                        result.push_str("test_resource");
                        result.push_str(test_res.path.as_str());
                        if let Some(ref query) = test_res.query {
                            result.push_str("?");
                            result.push_str(query);
                        }
                    }
                    TopicData::BlockchainHeight(_) => {
                        result.push_str("blockchain_height");
                    }
                    TopicData::Transaction(Transaction::ByAddress(tx)) => {
                        result.push_str(&format!(
                            "transactions?type={}&address={}",
                            tx.tx_type, tx.address
                        ));
                    }
                    TopicData::Transaction(Transaction::Exchange(tx)) => {
                        result.push_str(&format!(
                            "transactions?type=exchange&amount_asset={}&price_asset={}",
                            tx.amount_asset, tx.price_asset
                        ));
                    }
                    TopicData::LeasingBalance(lb) => {
                        result.push_str("leasing_balance/");
                        result.push_str(lb.address.as_str());
                    }
                    TopicData::Pairs(pairs) => {
                        result.push_str("pairs");
                        if pairs.len() == 1 {
                            result.push_str(&format!(
                                "/{}/{}",
                                pairs[0].amount_asset, pairs[0].price_asset
                            ));
                        } else {
                            result.push_str("/?pairs[]=");

                            let pairs = pairs
                                .iter()
                                .map(|p| format!("{}/{}", p.amount_asset, p.price_asset))
                                .collect::<Vec<String>>()
                                .join("&pairs[]=");

                            result.push_str(&pairs);
                        }
                    }
                }
                result
            }
        }
    }

    mod serde_state {
        use super::super::StateMultiPatterns;
        use serde::{Deserialize, Serialize};

        #[allow(non_snake_case)]
        #[derive(Deserialize)]
        struct Data {
            address__in: Vec<String>,
            key__match_any: Vec<String>,
        }

        #[allow(non_snake_case)]
        #[derive(Serialize)]
        struct DataRef<'a> {
            address__in: &'a [String],
            key__match_any: &'a [String],
        }

        pub(super) fn state_query_encode(v: &StateMultiPatterns) -> Result<String, ()> {
            let data = DataRef {
                address__in: &v.addresses,
                key__match_any: &v.key_patterns,
            };

            // Interestingly, this URL encoder does not replace '*' with '%2A' as per RFC-3986:
            // https://datatracker.ietf.org/doc/html/rfc3986#section-2.2
            // Same is for square brackets, '[' and ']'.
            // Though, it does not introduce any ambiguities or errors, so we're fine here.
            serde_qs::to_string(&data).map_err(|_| ())
        }

        pub(super) fn state_query_decode(s: &str) -> Result<StateMultiPatterns, ()> {
            let data: Data = serde_qs::from_str(s).map_err(|_| ())?;
            Ok(StateMultiPatterns {
                addresses: data.address__in,
                key_patterns: data.key__match_any,
            })
        }
    }

    mod url_escape {
        use percent_encoding::{
            percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC,
        };
        use std::borrow::Cow;

        const ENCODABLE_SET: AsciiSet = NON_ALPHANUMERIC.remove(b'_');

        pub(super) fn encode(s: &str) -> Cow<str> {
            utf8_percent_encode(s, &ENCODABLE_SET).into()
        }

        pub(super) fn decode(s: &str) -> Cow<str> {
            percent_decode_str(s).decode_utf8_lossy()
        }
    }

    pub mod maybe_string {
        use std::fmt;

        #[derive(Clone, PartialEq, Eq)]
        pub struct MaybeString(pub Option<String>);

        impl MaybeString {
            pub(super) fn from_emptyable_str(s: &str) -> Self {
                if s.is_empty() {
                    MaybeString(None)
                } else {
                    MaybeString(Some(s.to_owned()))
                }
            }
        }

        impl fmt::Debug for MaybeString {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.format(f)
            }
        }

        impl fmt::Display for MaybeString {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.format(f)
            }
        }

        impl MaybeString {
            fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0.as_ref() {
                    None => write!(f, "<Empty>"),
                    Some(s) => write!(f, "'{}'", s.as_str()),
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::Topic;

        #[test]
        fn topic_convert_test() -> anyhow::Result<()> {
            let urls = [
                "topic://config/some/path",
                "topic://state/address/key",
                "topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern2",
                "topic://test_resource/some/path?and_query=true",
                "topic://blockchain_height",
                "topic://transactions?type=all&address=some_address",
                "topic://transactions?type=exchange&amount_asset=foo&price_asset=bar",
                "topic://leasing_balance/some_address",
                "topic://pairs/amount_asset/price_asset",
                "topic://pairs/?pairs[]=amount_asset/price_asset&pairs[]=amount_asset1/price_asset1",
            ];
            for s in urls {
                let topic = Topic::parse_str(s)?;
                let other_s: String = topic.data().as_uri_string();
                assert_eq!(*s, other_s);
            }
            Ok(())
        }

        #[test]
        fn test_is_multi_topic() -> anyhow::Result<()> {
            let test_cases = [
                ("topic://config/some/path", false),
                ("topic://state/address/key", false),
                ("topic://state?address__in[]=address&key__match_any[]=key", true),
                ("topic://state?address__in[]=a1&address__in[]=a2&key__match_any[]=p1&key__match_any[]=pattern2", true),
                ("topic://test_resource/some/path?and_query=true", false),
                ("topic://blockchain_height", false),
                ("topic://transactions?type=all&address=some_address", false),
                ("topic://transactions?type=exchange&amount_asset=a&price_asset=p", false),
                ("topic://leasing_balance/some_address", false),
                ("topic://pairs/amount_asset/price_asset", false),
                ("topic://pairs?pairs[]=amount_asset/price_asset&pairs[]=amount_asset1/price_asset1", false),

            ];
            for (topic_url, expected_result) in test_cases {
                let topic = Topic::parse_str(topic_url)?;
                assert_eq!(
                    topic.is_multi_topic(),
                    expected_result,
                    "Failed: {}",
                    topic_url
                );
                assert_eq!(
                    topic.data().is_multi_topic(),
                    expected_result,
                    "Failed: {}",
                    topic_url
                );
            }
            Ok(())
        }
    }
}

impl Topic {
    pub fn kind(&self) -> TopicKind {
        // This is checked by `validate()` during parse stage, so `expect()` is safe
        let topic_kind_str = self.topic_url.host_str().expect("invariant broken: host");
        // Same safety guarantee
        TopicKind::parse(topic_kind_str).expect("invariant broken: topic_kind")
    }

    /// Whether this topic can be expanded to a set of other topics.
    pub fn is_multi_topic(&self) -> bool {
        self.kind() == TopicKind::State && self.topic_url.query().is_some()
    }

    pub fn data(&self) -> TopicData {
        TopicData::parse(self)
    }
}

impl TopicData {
    /// Whether this topic can be expanded to a set of other topics.
    pub fn is_multi_topic(&self) -> bool {
        match self {
            TopicData::State(State::MultiPatterns(_)) => true,
            _ => false,
        }
    }

    pub fn as_config(&self) -> Option<&ConfigResource> {
        match self {
            TopicData::Config(config) => Some(config),
            _ => None,
        }
    }

    pub fn as_state(&self) -> Option<&State> {
        match self {
            TopicData::State(state) => Some(state),
            _ => None,
        }
    }

    pub fn as_state_single(&self) -> Option<&StateSingle> {
        match self {
            TopicData::State(State::Single(state_single)) => Some(state_single),
            _ => None,
        }
    }

    pub fn as_state_multi(&self) -> Option<&StateMultiPatterns> {
        match self {
            TopicData::State(State::MultiPatterns(state_multi)) => Some(state_multi),
            _ => None,
        }
    }

    pub fn as_test_resource(&self) -> Option<&TestResource> {
        match self {
            TopicData::TestResource(test_res) => Some(test_res),
            _ => None,
        }
    }

    pub fn as_blockchain_height(&self) -> Option<&BlockchainHeight> {
        match self {
            TopicData::BlockchainHeight(blockchain_height) => Some(blockchain_height),
            _ => None,
        }
    }

    pub fn as_transaction(&self) -> Option<&Transaction> {
        match self {
            TopicData::Transaction(transaction) => Some(transaction),
            _ => None,
        }
    }

    pub fn as_leasing_balance(&self) -> Option<&LeasingBalance> {
        match self {
            TopicData::LeasingBalance(leasing_balance) => Some(leasing_balance),
            _ => None,
        }
    }

    pub fn as_pairs(&self) -> Option<&Vec<ExchangePairs>> {
        match self {
            TopicData::Pairs(pairs) => Some(&pairs),
            _ => None,
        }
    }

    pub fn as_topic(&self) -> Topic {
        let uri = self.as_uri_string();
        Topic::parse_str(&uri).expect("internal error: can't parse URI created from TopicData")
    }
}

#[test]
fn test_eq_and_hash() -> anyhow::Result<()> {
    let hash = |topic: &Topic| {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let mut hasher = DefaultHasher::new();
        topic.hash(&mut hasher);
        hasher.finish()
    };
    let topic_urls = [
        "topic://config/some/path",
        "topic://state/address/key",
        "topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern2",
        "topic://test_resource/some/path?and_query=true",
        "topic://blockchain_height",
        "topic://transactions?type=all&address=some_address",
        "topic://transactions?type=exchange&amount_asset=foo&price_asset=bar",
        "topic://leasing_balance/some_address",
    ];
    for topic_url in topic_urls {
        let topic1 = Topic::parse_str(topic_url)?;
        let topic2 = Topic::parse_str(topic_url)?;

        assert_eq!(topic1.to_string(), topic2.to_string());
        assert_eq!(topic1, topic2);

        let hash1 = hash(&topic1);
        let hash2 = hash(&topic2);
        assert_eq!(hash1, hash2);
    }
    Ok(())
}

mod convert {
    use super::{
        BlockchainHeight, ConfigFile, ConfigResource, LeasingBalance, State, StateMultiPatterns,
        StateSingle, TestResource, TopicData, Transaction, TransactionByAddress,
        TransactionExchange,
    };

    impl Into<TopicData> for ConfigResource {
        fn into(self) -> TopicData {
            TopicData::Config(self)
        }
    }

    impl Into<TopicData> for ConfigFile {
        fn into(self) -> TopicData {
            TopicData::Config(ConfigResource { file: self })
        }
    }

    impl Into<TopicData> for State {
        fn into(self) -> TopicData {
            TopicData::State(self)
        }
    }

    impl Into<TopicData> for StateSingle {
        fn into(self) -> TopicData {
            TopicData::State(State::Single(self))
        }
    }

    impl Into<TopicData> for StateMultiPatterns {
        fn into(self) -> TopicData {
            TopicData::State(State::MultiPatterns(self))
        }
    }

    impl Into<TopicData> for TestResource {
        fn into(self) -> TopicData {
            TopicData::TestResource(self)
        }
    }

    impl Into<TopicData> for BlockchainHeight {
        fn into(self) -> TopicData {
            TopicData::BlockchainHeight(self)
        }
    }

    impl Into<TopicData> for Transaction {
        fn into(self) -> TopicData {
            TopicData::Transaction(self)
        }
    }

    impl Into<TopicData> for TransactionByAddress {
        fn into(self) -> TopicData {
            TopicData::Transaction(Transaction::ByAddress(self))
        }
    }

    impl Into<TopicData> for TransactionExchange {
        fn into(self) -> TopicData {
            TopicData::Transaction(Transaction::Exchange(self))
        }
    }

    impl Into<TopicData> for LeasingBalance {
        fn into(self) -> TopicData {
            TopicData::LeasingBalance(self)
        }
    }
}
