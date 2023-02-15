//! Matcher client integration tests

mod mainnet {
    use bigdecimal::BigDecimal;
    use wavesexchange_apis::{HttpClient, Matcher};

    const MAINNET_MATCHER_API_URL: &str = "https://matcher.waves.exchange/matcher/settings/rates";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn assets_from_matcher() {
        let resp = HttpClient::<Matcher>::from_base_url(MAINNET_MATCHER_API_URL)
            .get()
            .await
            .unwrap();
        assert_eq!(resp["WAVES"], BigDecimal::from(1));
    }
}
