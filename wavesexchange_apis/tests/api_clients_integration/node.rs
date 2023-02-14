use crate::common::{MAINNET, TESTNET};
use wavesexchange_apis::{models::dto::DataEntry, node::dto, HttpClient, Node};

#[test_with::env(INTEGRATION)]
#[tokio::test]
async fn data_entries() {
    let keys: Vec<String> = ["UAH", "EUR", "CNY", "JPY", "RUB", "NGN"]
        .iter()
        .map(|sym| format!("%s%s__price__{}", sym))
        .collect();

    let mut data_entries = HttpClient::<Node>::from_base_url(MAINNET::node_url)
        .data_entries(MAINNET::defo_control_contract, keys)
        .await
        .unwrap();

    assert_eq!(data_entries.len(), 6);
    assert_eq!(
        DataEntry::from(data_entries.remove(0)).key,
        "%s%s__price__UAH"
    );
}

#[test_with::env(INTEGRATION)]
#[tokio::test]
async fn evaluate() {
    let result = HttpClient::<Node>::from_base_url(TESTNET::node_url)
        .evaluate(
            &TESTNET::products[0].contract_address,
            "privateCurrentSysParamsREST(\"5Sh9KghfkZyhjwuodovDhB6PghDUGBHiAPZ4MkrPgKtX\")",
        )
        .await
        .unwrap();

    match result.result {
        dto::Value::Tuple { value } => {
            let price = match value.get("_1") {
                Some(dto::Value::IntegerEntry {
                    value:
                        dto::IntegerEntryValue {
                            value: dto::IntValue { value },
                            ..
                        },
                }) => value.to_owned(),
                _ => panic!(),
            };

            let decimals_mult = match value.get("_2") {
                Some(dto::Value::IntegerEntry {
                    value:
                        dto::IntegerEntryValue {
                            value: dto::IntValue { value },
                            ..
                        },
                }) => value.to_owned(),
                _ => panic!(),
            };

            assert!(price > 0);
            assert!(decimals_mult > 0);
        }
        _ => panic!(),
    };
}
