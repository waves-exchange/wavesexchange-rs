use wavesexchange_apis::{mainnet_client, Levex};

#[tokio::test]
async fn test_levex_summary() {
    assert!(mainnet_client::<Levex>()
        .leveraged_tokens_summary()
        .await
        .is_ok());
}
