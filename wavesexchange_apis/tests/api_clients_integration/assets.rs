use wavesexchange_apis::{assets::dto, mainnet_client, AssetsService};

#[tokio::test]
async fn test_assets_get() {
    let resp = mainnet_client::<AssetsService>()
        .get(vec!["WAVES"], Some(1), dto::OutputFormat::Full, true)
        .await
        .unwrap();
    let resp = &resp.data[0];
    let data = if let dto::AssetInfo::Full(r) = resp.data.as_ref().unwrap() {
        r
    } else {
        panic!("Wrong output format");
    };
    assert_eq!(&data.id, "WAVES");
    assert_eq!(data.quantity, 10000000000000000);
    let label = &resp.metadata.as_ref().unwrap().labels[0];
    assert!(matches!(label, dto::AssetLabel::Gateway));
}
