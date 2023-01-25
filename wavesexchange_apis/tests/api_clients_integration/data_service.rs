use crate::common::MAINNET;
use chrono::{DateTime, NaiveDate, Utc};
use wavesexchange_apis::{data_service::dto, DataService, HttpClient};

const WAVES: &str = "WAVES";
const BTC: &str = "8LQW8f7P5d5PZM7GtZEBgaqRPGSzS3DfPuiXrURJ4AJS";
const NON_TRADABLE_ASSET: &str = "Ej5j5kr1hA4MmdKnewGgG7tJbiHFzotU2x2LELzHjW4o";
const USDN_ASSET_ID: &str = "DG2xFkPdDwKUoBkzGAhQtLpSGzfXLiCYPEzeKH2Ad24p";

#[tokio::test]
async fn fetch_rates_batch_from_data_service() {
    let rates = HttpClient::<DataService>::from_base_url(MAINNET::data_service_url)
        .rates(
            MAINNET::matcher,
            vec![(WAVES, BTC), (NON_TRADABLE_ASSET, WAVES)],
            None,
        )
        .await
        .unwrap()
        .data;

    assert_eq!(rates.len(), 2);
    assert!(rates[0].data.rate > 0.0);
    assert!(rates[1].data.rate == 0.0);
}

#[tokio::test]
async fn fetch_invokes_control_contract_finalize_current_price_v2() {
    // example invoke TS: 2021-06-21T16:38:52
    let timestamp_lt = NaiveDate::from_ymd_opt(2021, 06, 21)
        .unwrap()
        .and_hms_opt(16, 38, 53)
        .unwrap();

    let invokes = HttpClient::<DataService>::from_base_url(MAINNET::data_service_url)
        .invoke_script_transactions(
            None::<Vec<String>>,
            None,
            Some(timestamp_lt),
            Some(MAINNET::defo_control_contract),
            Some("finalizeCurrentPriceV2"),
            None::<String>,
            Some(dto::Sort::Desc),
            3,
        )
        .await
        .unwrap()
        .items;

    assert_eq!(invokes.len(), 3);

    let tx = &invokes[0].data;
    assert_eq!(tx.id, "2i6b9EksSrVS4dpX7LNxDisuR4JAgoNaCQSmXK1Gjwia");
    assert_eq!(tx.height, 2645143);
    assert_eq!(tx.proofs, ["4Msv4pGbR8wnmdHtzoWe6cr6eMRnmzBfNANX7jEpVDxKme8cQqtNzu9CYc4JUvhh52DmPpiuWCpe1DHN2ZGCruoG"]);
    assert_eq!(tx.version, 1);
    assert_eq!(tx.sender, "3PHYJhQ7WzGqvrPrbE5YKutSGHo4K2JRg42");
    assert_eq!(
        tx.sender_public_key,
        "8TLsCqkkroVot9dVR1WcWUN9Qx96HDfzG3hnx7NpSJA9"
    );
    assert_eq!(tx.fee, 0.005);

    // args with updated prices
    match &tx.call.args[1] {
        dto::InvokeScriptArgumentResponse::Binary { .. } => panic!(),
        dto::InvokeScriptArgumentResponse::String {
            value: updated_prices,
        } => assert_eq!(
            updated_prices,
            "BRL_5034198_1_UAH_27269793_1_GBP_718250_1_TRY_8764295_1"
        ),
    }
}

#[tokio::test]
async fn get_exchange_transactions() {
    let date1 = DateTime::from_utc(
        NaiveDate::from_ymd_opt(2021, 05, 01)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );
    let date2 = DateTime::from_utc(
        NaiveDate::from_ymd_opt(2021, 05, 01)
            .unwrap()
            .and_hms_opt(0, 30, 0)
            .unwrap(),
        Utc,
    );

    let txs_resp = HttpClient::<DataService>::from_base_url(MAINNET::data_service_url)
        .transactions_exchange(
            Option::<String>::None,
            Option::<String>::None,
            Some(WAVES),
            Some(USDN_ASSET_ID),
            Some(date1),
            Some(date2),
            dto::Sort::Desc,
            3,
            Option::<String>::None,
        )
        .await
        .unwrap();

    assert_eq!(txs_resp.items.len(), 3);
}
