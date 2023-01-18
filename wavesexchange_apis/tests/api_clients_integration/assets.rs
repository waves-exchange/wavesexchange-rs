use crate::common::MAINNET;
use wavesexchange_apis::{assets::dto, AssetsService, HttpClient};

#[tokio::test]
async fn test_assets_get() {
    let resp = HttpClient::<AssetsService>::from_base_url(MAINNET::assets_service_url)
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
