use crate::common::MAINNET;
use wavesexchange_apis::{HttpClient, Levex};

#[tokio::test]
async fn test_levex_summary() {
    assert!(HttpClient::<Levex>::from_base_url(MAINNET::levex_api_url)
        .leveraged_tokens_summary()
        .await
        .is_ok());
}
