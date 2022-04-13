use chrono::{Date, NaiveDate, Utc};
use wavesexchange_apis::test_configs::blockchains::MAINNET;
use wavesexchange_apis::{data_service::dto, mainnet_client, DataService};

const WAVES: &str = "WAVES";
const BTC: &str = "8LQW8f7P5d5PZM7GtZEBgaqRPGSzS3DfPuiXrURJ4AJS";
const NON_TRADABLE_ASSET: &str = "Ej5j5kr1hA4MmdKnewGgG7tJbiHFzotU2x2LELzHjW4o";
const USDN_ASSET_ID: &str = "DG2xFkPdDwKUoBkzGAhQtLpSGzfXLiCYPEzeKH2Ad24p";

#[tokio::test]
async fn fetch_rates_batch_from_data_service() {
    let rates = mainnet_client::<DataService>()
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
    let timestamp_lt = NaiveDate::from_ymd(2021, 06, 21).and_hms(16, 38, 53);

    let invokes = mainnet_client::<DataService>()
        .invoke_script_transactions(
            MAINNET::defo_control_contract,
            "finalizeCurrentPriceV2",
            timestamp_lt,
            dto::Sort::Desc,
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
    let date = Date::from_utc(NaiveDate::from_ymd(2021, 05, 01), Utc);

    let txs_resp = mainnet_client::<DataService>()
        .transactions_exchange(
            Option::<String>::None,
            Some(WAVES),
            Some(USDN_ASSET_ID),
            Some(date.and_hms(0, 0, 0)),
            Some(date.and_hms(0, 30, 0)),
            dto::Sort::Desc,
            3,
            Option::<String>::None,
        )
        .await
        .unwrap();

    assert_eq!(txs_resp.items.len(), 3);
}
