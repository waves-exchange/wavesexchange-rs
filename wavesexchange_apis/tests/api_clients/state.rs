use serde_json::json;
use wavesexchange_apis::{mainnet_client, testnet_client, StateService};

#[tokio::test]
async fn test_get_state() {
    let entries = testnet_client::<StateService>()
        .entries(
            "3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To",
            "%s__priceDecimals",
            None,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entries.key, "%s__priceDecimals");

    let entries_none = testnet_client::<StateService>()
        .entries("3MrbnZkriTBZhRqS45L1VfCrden6Erpa7To", "%s__priceDeci", None)
        .await
        .unwrap();
    assert!(entries_none.is_none());
}

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
                    ["3P8qJyxUqizCWWtEn2zsLZVPzZAjdNGppB1", "%s%s__price__UAH"]
                ]
            }
        }
    });

    let entries = mainnet_client::<StateService>()
        .search(query, None, None)
        .await
        .unwrap();

    assert_eq!(entries.items.len(), 1);
}

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

    let entries = mainnet_client::<StateService>()
        .search(query, None, None)
        .await
        .unwrap();

    assert!(entries.items.len() >= 9);
}
