//! State Service client integration tests

mod mainnet {
    use serde_json::json;
    use wavesexchange_apis::{HttpClient, StateService};

    const MAINNET_STATE_SERVICE_URL: &str = "https://waves.exchange/api/v1/state";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn single_asset_price_request() {
        let query = json!({
            "filter": {
                "in": {
                    "properties": [
                        {
                            "address": {}
                        },
                        {
                            "key": {}
                        }
                    ],
                    "values": [
                        ["3P3hCvE9ZfeMnZE6kXzR6YBzxhxM8J6PE7K", "%s%s%d__total__locked__0"]
                    ]
                }
            }
        });

        let entries = HttpClient::<StateService>::from_base_url(MAINNET_STATE_SERVICE_URL)
            .search(query, None, None)
            .await
            .unwrap();

        assert_eq!(entries.items.len(), 1);
    }

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn defo_assets_list() {
        let query = json!({
            "filter": {
                "and": [
                  {
                    "address": {
                      "value": "3PQEjFmdcjd6wf1TrpkHSuDAk3zbfLSeikb"
                    }
                  },
                  {
                    "fragment": {
                      "position": 0,
                      "type": "string",
                      "operation": "eq",
                      "value": "defoAsset"
                    }
                  },
                  {
                    "fragment": {
                      "position": 2,
                      "type": "string",
                      "operation": "eq",
                      "value": "config"
                    }
                  }
                ]
            }
        });

        let entries = HttpClient::<StateService>::from_base_url(MAINNET_STATE_SERVICE_URL)
            .search(query, None, None)
            .await
            .unwrap();

        assert!(entries.items.len() >= 9);
    }
}

mod testnet {
    use wavesexchange_apis::{HttpClient, StateService};

    const TESTNET_STATE_SERVICE_URL: &str = "https://testnet.waves.exchange/api/v1/state";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn get_state() {
        let client = HttpClient::<StateService>::from_base_url(TESTNET_STATE_SERVICE_URL);
        let entries = client
            .entries(
                "3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To",
                "%s__priceDecimals",
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entries.key, "%s__priceDecimals");

        let entries_none = client
            .entries("3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To", "%s__priceDeci", None)
            .await
            .unwrap();
        assert!(entries_none.is_none());
    }
}
