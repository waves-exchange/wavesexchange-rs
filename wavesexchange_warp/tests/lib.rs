use std::convert::Infallible;
use std::time::Duration;

use tokio::{spawn, time};
use warp::Filter;
use wavesexchange_warp::MetricsWarpBuilder;

#[tokio::test]
async fn test_run_metrics_warp() {
    let main_port = 8081;
    let metrics_port = 9001;
    let url = format!("http://0.0.0.0:{main_port}");
    let metrics_url = format!("http://0.0.0.0:{}", metrics_port);
    let routes = warp::path!("hello").and_then(|| async { Ok::<_, Infallible>("Hello, world!") });

    let warps = MetricsWarpBuilder::new()
        .with_main_routes(routes)
        .with_startz_checker(|| async { Err("still not enough racoons") })
        .with_metrics_port(metrics_port)
        .with_main_routes_port(main_port)
        .run_blocking();

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

    let startz_check = reqwest::get(format!("{metrics_url}/startz"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(startz_check.contains("still not enough racoons"));

    let metrics = reqwest::get(format!("{metrics_url}/metrics"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("{metrics}");

    // don't count requests to metrics_url
    assert!(metrics.contains("incoming_requests 2"));
    assert!(metrics.contains(r#"response_duration_count{code="200",method="GET"} 1"#));
    assert!(metrics.contains(r#"response_duration_count{code="404",method="GET"} 1"#));
}

#[tokio::test]
async fn test_graceful_shutdown() {
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel::<()>();
    let main_port = 8081;
    let url = format!("http://0.0.0.0:{main_port}");
    let routes = warp::path!("hello").and_then(|| async { Ok::<_, Infallible>("Hello, world!") });

    let warps = MetricsWarpBuilder::new()
        .with_main_routes(routes)
        .with_graceful_shutdown(async {
            let _ = rx.await.unwrap();
        })
        .with_main_routes_port(main_port)
        .run_blocking();

    spawn(warps);
    time::sleep(Duration::from_secs(1)).await; // wait for server

    let hello = reqwest::get(format!("{url}/hello"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(hello, "Hello, world!");

    tx.send(()).unwrap();

    let error = reqwest::get(format!("{url}/hello")).await.unwrap_err();
    assert!(error.is_connect());
}
