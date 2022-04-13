use wavesexchange_apis::{
    mainnet_client,
    models::dto::DataEntry,
    node::dto,
    test_configs::blockchains::{MAINNET, TESTNET},
    testnet_client, Node,
};

#[tokio::test]
async fn data_entries() {
    let keys: Vec<String> = ["UAH", "EUR", "CNY", "JPY", "RUB", "NGN"]
        .iter()
        .map(|sym| format!("%s%s__price__{}", sym))
        .collect();

    let mut data_entries = mainnet_client::<Node>()
        .data_entries(MAINNET::defo_control_contract, keys)
        .await
        .unwrap();

    assert_eq!(data_entries.len(), 6);
    assert_eq!(
        DataEntry::from(data_entries.remove(0)).key,
        "%s%s__price__UAH"
    );
}

#[tokio::test]
async fn evaluate() {
    let result = testnet_client::<Node>()
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
