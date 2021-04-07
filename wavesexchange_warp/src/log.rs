use wavesexchange_log::info;

pub fn access(info: warp::log::Info) {
    let headers = info.request_headers();

    let req_id = headers
        .get("x-request-id")
        .map(|h| h.to_str().unwrap_or(&""));

    // info.remote_addr stores the proxy ip, not client
    let ip = headers.get("x-real-ip").map(|h| h.to_str().unwrap_or(&""));

    info!(
        "access";
        "path" => info.path(),
        "method" => info.method().to_string(),
        "status" => info.status().as_u16(),
        "ua" => info.user_agent(),
        "latency" => info.elapsed().as_millis() as u64,
        "req_id" => req_id,
        "ip" => ip,
        "protocol" => format!("{:?}", info.version())
    );
}
