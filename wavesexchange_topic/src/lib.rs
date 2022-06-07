pub mod error;

use error::Error;
use std::{convert::TryFrom, str::FromStr};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    Config(ConfigParameters),
    State(State),
    TestResource(TestResource),
    BlockchainHeight,
    Transaction(Transaction),
    LeasingBalance(LeasingBalance),
}

#[test]
fn topic_convert_test() {
    let urls = [
        "topic://config/some/path",
        "topic://state/address/key",
        "topic://state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern2",
        "topic://test_resource/some/path?and_query=true",
        "topic://blockchain_height",
        "topic://transactions?type=all&address=some_address",
        "topic://transactions?type=exchange&amount_asset=foo&price_asset=bar",
        "topic://leasing_balance/some_address",
    ];
    for s in urls.iter() {
        let topic = Topic::try_from(*s).unwrap();
        let other_s: String = topic.into();
        assert_eq!(*s, other_s);
    }
}

impl Topic {
    /// Whether this topic can be expanded to a set of other topics.
    pub fn is_multi_topic(&self) -> bool {
        match self {
            Topic::State(State::MultiPatterns(_)) => true,
            _ => false,
        }
    }
}

#[test]
fn topic_wildcard_test() {
    let test_cases = [
        ("topic://config/some/path", false),
        ("topic://state/address/key", false),
        ("topic://state?address__in[]=address&key__match_any[]=key", true),
        (
            "topic://state?address__in[]=a1&address__in[]=a2&key__match_any[]=p1&key__match_any[]=pattern2",
            true,
        ),
        ("topic://test_resource/some/path?and_query=true", false),
        ("topic://blockchain_height", false),
        ("topic://transactions?type=all&address=some_address", false),
        (
            "topic://transactions?type=exchange&amount_asset=a&price_asset=p",
            false,
        ),
        ("topic://leasing_balance/some_address", false),
    ];
    for (topic_url, expected_result) in test_cases {
        let topic = Topic::try_from(topic_url).unwrap();
        assert_eq!(
            topic.is_multi_topic(),
            expected_result,
            "Failed: {}",
            topic_url
        );
    }
}

impl TryFrom<&str> for Topic {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(s)?;

        match url.host_str() {
            Some("config") => {
                let file = ConfigFile::try_from(url)?;
                Ok(Topic::Config(ConfigParameters { file }))
            }
            Some("state") => {
                let state = State::try_from(url)?;
                Ok(Topic::State(state))
            }
            Some("test_resource") => {
                let ps = TestResource::try_from(url)?;
                Ok(Topic::TestResource(ps))
            }
            Some("blockchain_height") => Ok(Topic::BlockchainHeight),
            Some("transactions") => {
                let transaction = Transaction::try_from(url)?;
                Ok(Topic::Transaction(transaction))
            }
            Some("leasing_balance") => {
                let leasing_balance = LeasingBalance::try_from(url)?;
                Ok(Topic::LeasingBalance(leasing_balance))
            }
            _ => Err(Error::InvalidTopic(s.to_owned())),
        }
    }
}

impl From<Topic> for String {
    fn from(v: Topic) -> String {
        let mut result = "topic://".to_string();
        match v {
            Topic::Config(cp) => result.push_str(&String::from(cp)),
            Topic::State(state) => result.push_str(&String::from(state)),
            Topic::TestResource(ps) => result.push_str(&String::from(ps)),
            Topic::BlockchainHeight => result.push_str("blockchain_height"),
            Topic::Transaction(tx) => result.push_str(&String::from(tx)),
            Topic::LeasingBalance(leasing_balance) => {
                result.push_str(&String::from(leasing_balance))
            }
        }
        result
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConfigFile {
    pub path: String,
}

impl TryFrom<Url> for ConfigFile {
    type Error = Error;

    fn try_from(u: Url) -> Result<Self, Self::Error> {
        Ok(ConfigFile {
            path: u.path().to_owned(),
        })
    }
}

impl From<ConfigFile> for String {
    fn from(v: ConfigFile) -> String {
        "config".to_owned() + &v.path
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConfigParameters {
    pub file: ConfigFile,
}

impl From<ConfigParameters> for String {
    fn from(v: ConfigParameters) -> String {
        v.file.into()
    }
}

impl TryFrom<Url> for ConfigParameters {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let config_file = ConfigFile::try_from(value)?;
        Ok(Self { file: config_file })
    }
}

impl From<ConfigFile> for Topic {
    fn from(v: ConfigFile) -> Self {
        let cp = ConfigParameters { file: v };
        cp.into()
    }
}

impl From<ConfigParameters> for Topic {
    fn from(v: ConfigParameters) -> Self {
        Self::Config(v)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Single(StateSingle),
    MultiPatterns(StateMultiPatterns),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateSingle {
    pub address: String,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateMultiPatterns {
    pub addresses: Vec<String>,
    pub key_patterns: Vec<String>,
}

mod serde_state {
    use super::StateMultiPatterns;
    use serde::{Deserialize, Serialize};

    #[allow(non_snake_case)]
    #[derive(Deserialize, Serialize)]
    struct Data {
        address__in: Vec<String>,
        key__match_any: Vec<String>,
    }

    pub(super) fn state_query_encode(v: StateMultiPatterns) -> Result<String, ()> {
        let data = Data {
            address__in: v.addresses,
            key__match_any: v.key_patterns,
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
    use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
    use std::borrow::Cow;

    const ENCODABLE_SET: AsciiSet = NON_ALPHANUMERIC.remove(b'_');

    pub(super) fn encode(s: &str) -> Cow<str> {
        utf8_percent_encode(s, &ENCODABLE_SET).into()
    }

    pub(super) fn decode(s: &str) -> Cow<str> {
        percent_decode_str(s).decode_utf8_lossy()
    }
}

impl From<State> for String {
    fn from(v: State) -> String {
        match v {
            State::Single(single) => single.into(),
            State::MultiPatterns(multi) => multi.into(),
        }
    }
}

impl From<StateSingle> for String {
    fn from(v: StateSingle) -> String {
        let address = url_escape::encode(&v.address);
        let key = url_escape::encode(&v.key);
        format!("state/{}/{}", address, key)
    }
}

impl From<StateMultiPatterns> for String {
    fn from(v: StateMultiPatterns) -> String {
        "state?".to_string() + &serde_state::state_query_encode(v).unwrap()
    }
}

impl TryFrom<Url> for State {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        Ok(if value.query().is_none() {
            Self::Single(StateSingle::try_from(value)?)
        } else {
            Self::MultiPatterns(StateMultiPatterns::try_from(value)?)
        })
    }
}

impl TryFrom<Url> for StateSingle {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let params = value
            .path_segments()
            .ok_or_else(|| Error::InvalidStatePath(value.path().to_string()))?
            .take(2)
            .collect::<Vec<_>>();
        if params.len() == 2 {
            let address = url_escape::decode(params[0]).into_owned();
            let key = url_escape::decode(params[1]).into_owned();
            Ok(Self { address, key })
        } else {
            Err(Error::InvalidStatePath(value.path().to_string()))
        }
    }
}

impl TryFrom<Url> for StateMultiPatterns {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        use crate::error::ErrorQuery;
        let query = value
            .query()
            .ok_or_else(|| Error::InvalidStateQuery(ErrorQuery(None)))?;
        serde_state::state_query_decode(query)
            .map_err(|_| Error::InvalidStateQuery(ErrorQuery(Some(query.to_owned()))))
    }
}

impl From<State> for Topic {
    fn from(v: State) -> Self {
        Self::State(v)
    }
}

impl From<StateSingle> for Topic {
    fn from(v: StateSingle) -> Self {
        Self::State(State::Single(v))
    }
}

impl From<StateMultiPatterns> for Topic {
    fn from(v: StateMultiPatterns) -> Self {
        Self::State(State::MultiPatterns(v))
    }
}

#[test]
fn topic_state_test() {
    let url = Url::parse("topic://state/some_address/some_key").unwrap();
    let state = State::try_from(url).unwrap();
    assert!(matches!(state, State::Single(_)));
    if let State::Single(ref state) = state {
        assert_eq!(state.address, "some_address".to_string());
        assert_eq!(state.key, "some_key".to_string());
    }

    let url = Url::parse("topic://state/some_address/some_key/some_other_part_of_path").unwrap();
    let state = State::try_from(url).unwrap();
    assert!(matches!(state, State::Single(_)));
    if let State::Single(ref state) = state {
        assert_eq!(state.address, "some_address".to_string());
        assert_eq!(state.key, "some_key".to_string());
    }
    let state_string: String = state.into();
    assert_eq!("state/some_address/some_key".to_string(), state_string);

    // URL with plain (not percent-encoded) character '*' should work
    let url =
        Url::parse("topic://state?address__in[]=addr1&address__in[]=addr2&key__match_any[]=pattern1&key__match_any[]=pattern*2").unwrap();
    let state = State::try_from(url).unwrap();
    assert!(matches!(state, State::MultiPatterns(_)));
    if let State::MultiPatterns(ref state) = state {
        assert_eq!(state.addresses, vec!["addr1", "addr2"]);
        assert_eq!(state.key_patterns, vec!["pattern1", "pattern*2"]);
    }
    let state_string: String = state.into();
    assert_eq!(
        "state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern*2".to_string(),
        state_string
    );

    // URL with properly percent-encoded chars should also work
    let url =
        Url::parse("topic://state?address__in[]=addr1&address__in[]=addr2&key__match_any[]=pattern1&key__match_any[]=pattern%2A2")
            .unwrap();
    let state = State::try_from(url).unwrap();
    assert!(matches!(state, State::MultiPatterns(_)));
    if let State::MultiPatterns(ref state) = state {
        assert_eq!(state.addresses, vec!["addr1", "addr2"]);
        assert_eq!(state.key_patterns, vec!["pattern1", "pattern*2"]);
    }
    let state_string: String = state.into();
    assert_eq!(
        "state?address__in[0]=addr1&address__in[1]=addr2&key__match_any[0]=pattern1&key__match_any[1]=pattern*2".to_string(),
        state_string
    );
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TestResource {
    pub path: String,
    pub query: Option<String>,
}

impl From<TestResource> for String {
    fn from(v: TestResource) -> String {
        let mut s = "test_resource".to_owned() + &v.path;
        if let Some(ref query) = v.query {
            s = s + "?" + query;
        }
        s
    }
}

impl TryFrom<Url> for TestResource {
    type Error = Error;

    fn try_from(u: Url) -> Result<Self, Self::Error> {
        Ok(Self {
            path: u.path().to_string(),
            query: u.query().map(|q| q.to_owned()),
        })
    }
}

impl From<TestResource> for Topic {
    fn from(v: TestResource) -> Self {
        Self::TestResource(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockchainHeight {}

impl TryFrom<Url> for BlockchainHeight {
    type Error = Error;

    fn try_from(_value: Url) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

impl From<BlockchainHeight> for String {
    fn from(_: BlockchainHeight) -> String {
        "blockchain_height".to_string()
    }
}

impl From<BlockchainHeight> for Topic {
    fn from(_: BlockchainHeight) -> Self {
        Self::BlockchainHeight
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Transaction {
    ByAddress(TransactionByAddress),
    Exchange(TransactionExchange),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransactionExchange {
    pub amount_asset: String,
    pub price_asset: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransactionByAddress {
    pub tx_type: TransactionType,
    pub address: String,
}

impl TryFrom<Url> for Transaction {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        if let Ok(raw_tx_type) = query_utils::get(&value, "type") {
            let tx_type = FromStr::from_str(raw_tx_type.as_str())?;
            match tx_type {
                TransactionType::Exchange => {
                    if let Ok(tx) = TransactionExchange::try_from(value.clone()) {
                        return Ok(Self::Exchange(tx));
                    }
                }
                _ => (),
            }
        }
        let tx = TransactionByAddress::try_from(value)?;
        Ok(Self::ByAddress(tx))
    }
}

impl TryFrom<Url> for TransactionByAddress {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let get_value_from_query =
            |url, key| query_utils::get(url, key).map_err(|e| Error::InvalidTransactionQuery(e));
        let tx_type = if let Ok(raw_tx_type) = get_value_from_query(&value, "type") {
            FromStr::from_str(raw_tx_type.as_str())?
        } else {
            TransactionType::All
        };
        let address = get_value_from_query(&value, "address")?;
        Ok(Self { tx_type, address })
    }
}

impl TryFrom<Url> for TransactionExchange {
    type Error = Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let get_value_from_query =
            |url, key| query_utils::get(url, key).map_err(|e| Error::InvalidTransactionQuery(e));
        let price_asset = get_value_from_query(&value, "price_asset")?;
        let amount_asset = get_value_from_query(&value, "amount_asset")?;
        Ok(Self {
            amount_asset,
            price_asset,
        })
    }
}

impl From<Transaction> for String {
    fn from(v: Transaction) -> String {
        match v {
            Transaction::ByAddress(by_address) => by_address.into(),
            Transaction::Exchange(exchange) => exchange.into(),
        }
    }
}

impl From<TransactionByAddress> for String {
    fn from(v: TransactionByAddress) -> String {
        format!("transactions?type={}&address={}", v.tx_type, v.address)
    }
}

impl From<TransactionExchange> for String {
    fn from(v: TransactionExchange) -> String {
        format!(
            "transactions?type=exchange&amount_asset={}&price_asset={}",
            v.amount_asset, v.price_asset
        )
    }
}

impl From<Transaction> for Topic {
    fn from(v: Transaction) -> Self {
        Self::Transaction(v)
    }
}

impl From<TransactionByAddress> for Topic {
    fn from(v: TransactionByAddress) -> Self {
        Self::Transaction(Transaction::ByAddress(v))
    }
}

impl From<TransactionExchange> for Topic {
    fn from(v: TransactionExchange) -> Self {
        Self::Transaction(Transaction::Exchange(v))
    }
}

#[test]
fn transaction_topic_test() {
    let url = Url::parse("topic://transactions?type=all&address=some_address").unwrap();
    if let Transaction::ByAddress(transaction) = Transaction::try_from(url).unwrap() {
        assert_eq!(transaction.tx_type.to_string(), "all".to_string());
        assert_eq!(transaction.address, "some_address".to_string());
        assert_eq!(
            "topic://transactions?type=all&address=some_address".to_string(),
            String::from(Topic::Transaction(Transaction::ByAddress(transaction)))
        );
    } else {
        panic!("wrong transaction")
    }
    let url = Url::parse("topic://transactions?type=issue&address=some_other_address").unwrap();
    if let Transaction::ByAddress(transaction) = Transaction::try_from(url).unwrap() {
        assert_eq!(transaction.tx_type.to_string(), "issue".to_string());
        assert_eq!(transaction.address, "some_other_address".to_string());
        assert_eq!(
            "topic://transactions?type=issue&address=some_other_address".to_string(),
            String::from(Topic::Transaction(Transaction::ByAddress(transaction)))
        );
    }
    let url = Url::parse("topic://transactions").unwrap();
    let error = Transaction::try_from(url);
    assert!(error.is_err());
    assert_eq!(
        format!("{}", error.unwrap_err()),
        "InvalidTransactionQuery: None".to_string()
    );
    let url =
        Url::parse("topic://transactions?type=exchange&amount_asset=asd&price_asset=qwe").unwrap();
    if let Transaction::Exchange(transaction) = Transaction::try_from(url).unwrap() {
        assert_eq!(transaction.amount_asset, "asd".to_string());
        assert_eq!(transaction.price_asset, "qwe".to_string());
        assert_eq!(
            "topic://transactions?type=exchange&amount_asset=asd&price_asset=qwe".to_string(),
            String::from(Topic::Transaction(Transaction::Exchange(transaction)))
        );
    } else {
        panic!("wrong exchange transaction")
    }
    let url =
        Url::parse("topic://transactions?type=exchange&amount_asset=asd&price_asset=").unwrap();
    let error = Transaction::try_from(url);
    assert!(error.is_err());
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl FromStr for TransactionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let transaction_type = match s {
            "all" => Self::All,
            "genesis" => Self::Genesis,
            "payment" => Self::Payment,
            "issue" => Self::Issue,
            "transfer" => Self::Transfer,
            "reissue" => Self::Reissue,
            "burn" => Self::Burn,
            "exchange" => Self::Exchange,
            "lease" => Self::Lease,
            "lease_cancel" => Self::LeaseCancel,
            "alias" => Self::Alias,
            "mass_transfer" => Self::MassTransfer,
            "data" => Self::Data,
            "set_script" => Self::SetScript,
            "sponsorship" => Self::Sponsorship,
            "set_asset_script" => Self::SetAssetScript,
            "invoke_script" => Self::InvokeScript,
            "update_asset_info" => Self::UpdateAssetInfo,
            "invoke_expression" => Self::InvokeExpression,
            _ => return Err(Error::InvalidTransactionType(s.to_string())),
        };
        Ok(transaction_type)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LeasingBalance {
    pub address: String,
}

impl From<LeasingBalance> for String {
    fn from(v: LeasingBalance) -> String {
        "leasing_balance/".to_string() + v.address.as_str()
    }
}

impl TryFrom<Url> for LeasingBalance {
    type Error = Error;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        let mut address = None;
        if let Some(mut path_segments) = url.path_segments() {
            if let Some(address_segment) = path_segments.next() {
                address = Some(address_segment.to_string())
            }
        }
        if let Some(address) = address {
            Ok(Self { address })
        } else {
            return Err(Error::InvalidLeasingPath(url.path().to_string()));
        }
    }
}

impl From<LeasingBalance> for Topic {
    fn from(v: LeasingBalance) -> Self {
        Self::LeasingBalance(v)
    }
}

#[test]
fn leasing_balance_test() {
    let url = Url::parse("topic://leasing_balance/some_address").unwrap();
    let leasing_balance = LeasingBalance::try_from(url).unwrap();
    assert_eq!(leasing_balance.address, "some_address".to_string());
    let url = Url::parse("topic://leasing_balance/some_address/some_other_part_of_path").unwrap();
    let leasing_balance = LeasingBalance::try_from(url).unwrap();
    assert_eq!(leasing_balance.address, "some_address".to_string());
    let leasing_balance_string: String = leasing_balance.into();
    assert_eq!(
        "leasing_balance/some_address".to_string(),
        leasing_balance_string
    );
}

mod query_utils {
    use crate::error::ErrorQuery;
    use url::Url;

    pub(super) fn get(value: &Url, key: &str) -> Result<String, ErrorQuery> {
        value
            .query_pairs()
            .find_map(|(k, v)| {
                if k == key && !v.is_empty() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| ErrorQuery(value.query().map(ToString::to_string)))
    }
}
