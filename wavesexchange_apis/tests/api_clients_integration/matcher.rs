use crate::common::MAINNET;
use bigdecimal::BigDecimal;
use wavesexchange_apis::{HttpClient, Matcher};

#[test_with::env(INTEGRATION)]
#[tokio::test]
async fn test_assets_from_matcher() {
    let resp = HttpClient::<Matcher>::from_base_url(MAINNET::matcher_api_url)
        .get()
        .await
        .unwrap();
    assert_eq!(resp["WAVES"], BigDecimal::from(1));
}
