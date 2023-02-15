//! Asset Service client integration tests

mod mainnet {
    use wavesexchange_apis::{assets::dto, AssetsService, HttpClient};

    const MAINNET_ASSETS_SERVICE_URL: &str = "https://waves.exchange/api/v1/assets";

    #[test_with::env(INTEGRATION)]
    #[tokio::test]
    async fn assets_get() {
        let resp = HttpClient::<AssetsService>::from_base_url(MAINNET_ASSETS_SERVICE_URL)
            .get(vec!["WAVES"], Some(1), dto::OutputFormat::Full, true)
            .await
            .unwrap();
        let resp = &resp.data[0];
        let dto::AssetInfo::Full(data) = resp.data.as_ref().unwrap() else {
            panic!("Wrong output format");
        };
        assert_eq!(&data.id, "WAVES");
        assert_eq!(data.quantity, 10000000000000000);
        let label = &resp.metadata.as_ref().unwrap().labels[0];
        assert!(matches!(label, dto::AssetLabel::Gateway));
    }
}
