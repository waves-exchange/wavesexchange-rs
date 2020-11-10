use wavesexchange_log::info;

pub fn access(info: warp::log::Info) {
    let req_id = info
        .request_headers()
        .get("x-request-id")
        .map(|h| h.to_str().unwrap_or(&""));

    info!(
        "access";
        "path" => info.path(),
        "method" => info.method().to_string(),
        "status" => info.status().as_u16(),
        "ua" => info.user_agent(),
        "latency" => info.elapsed().as_millis(),
        "req_id" => req_id,
        "ip" => info.remote_addr().map(|a| format!("{}", a.ip())),
        "protocol" => format!("{:?}", info.version())
    );
}
