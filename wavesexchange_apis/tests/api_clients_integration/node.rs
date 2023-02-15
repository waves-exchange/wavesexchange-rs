//! Node client integration tests

mod mainnet {
    use wavesexchange_apis::{models::dto::DataEntry, HttpClient, Node};

    const MAINNET_NODE_URL: &str = "https://nodes.waves.exchange";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn data_entries() {
        let keys: Vec<String> = ["UAH", "EUR", "CNY", "JPY", "RUB", "NGN"]
            .iter()
            .map(|sym| format!("%s%s__price__{}", sym))
            .collect();

        let defo_control_contract = "3P8qJyxUqizCWWtEn2zsLZVPzZAjdNGppB1";

        let mut data_entries = HttpClient::<Node>::from_base_url(MAINNET_NODE_URL)
            .data_entries(defo_control_contract, keys)
            .await
            .unwrap();

        assert_eq!(data_entries.len(), 6);
        assert_eq!(
            DataEntry::from(data_entries.remove(0)).key,
            "%s%s__price__UAH"
        );
    }
}

mod testnet {
    use wavesexchange_apis::{node::dto, HttpClient, Node};

    const TESTNET_NODE_URL: &str = "https://nodes-testnet.wavesnodes.com";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn evaluate() {
        let any_stake_contract_address = "3Mzt645zA6u2QG6jRPoo6H6CK89kVggFgNi";

        let result = HttpClient::<Node>::from_base_url(TESTNET_NODE_URL)
            .evaluate(
                any_stake_contract_address,
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
}
