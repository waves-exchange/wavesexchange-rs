use bigdecimal::BigDecimal;
use wavesexchange_apis::{mainnet_client, Matcher};

#[tokio::test]
async fn test_assets_from_matcher() {
    let resp = mainnet_client::<Matcher>().get().await.unwrap();
    assert_eq!(resp["WAVES"], BigDecimal::from(1));
}
