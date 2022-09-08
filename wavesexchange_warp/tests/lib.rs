use std::convert::Infallible;
use std::time::Duration;

use tokio::{spawn, time};
use warp::Filter;
use wavesexchange_warp::StatsWarpBuilder;

#[tokio::test]
async fn test_run_stats_warp() {
    let port = 8080;
    let stats_port = 9001;
    let url = format!("http://0.0.0.0:{port}");
    let stats_url = format!("http://0.0.0.0:{}", stats_port);
    let routes = warp::path!("hello").and_then(|| async { Ok::<_, Infallible>("Hello, world!") });

    let warps = StatsWarpBuilder::new()
        .with_main_routes(routes)
        .with_startz_checker(|| async { Err("still not enough racoons") })
        .with_stats_port(stats_port)
        .run_blocking(port);

    spawn(warps);
    time::sleep(Duration::from_secs(1)).await; // wait for server

    let hello = reqwest::get(format!("{url}/hello"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(hello, "Hello, world!");

    let not_found = reqwest::get(format!("{url}/not_found")).await.unwrap();
    assert_eq!(not_found.status().as_u16(), 404);

    let startz_check = reqwest::get(format!("{stats_url}/startz"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(startz_check.contains("still not enough racoons"));

    let metrics = reqwest::get(format!("{stats_url}/metrics"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{metrics}");

    // requests to stats_url don't count
    assert!(metrics.contains("incoming_requests 2"));
    assert!(metrics.contains(r#"response_duration_count{code="200",method="GET"} 1"#));
    assert!(metrics.contains(r#"response_duration_count{code="404",method="GET"} 1"#));
}
