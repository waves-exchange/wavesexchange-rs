use super::{livez, readyz, startz, HealthcheckReply, Shared};
use futures::future::join;
use lazy_static::lazy_static;
use prometheus::{HistogramOpts, HistogramVec, IntCounter, Registry, TextEncoder};
use warp::{filters::BoxedFilter, log::Info, Filter, Rejection, Reply};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").unwrap();
    static ref RESPONSE_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("response_duration", "Response duration in secs"),
        &["code", "method"]
    )
    .unwrap();
}

pub const STATS_PORT_OFFSET: u16 = 1010;

fn register_metrics() {
    REGISTRY.register(Box::new(REQUESTS.clone())).unwrap();
    REGISTRY
        .register(Box::new(RESPONSE_DURATION.clone()))
        .unwrap();
}

fn estimate_request(info: Info) {
    REQUESTS.inc();
    RESPONSE_DURATION
        .with_label_values(&[info.status().as_str(), info.method().as_str()])
        .observe(info.elapsed().as_secs_f64());
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = TextEncoder::new();
    let result = encoder.encode_to_string(&REGISTRY.gather()).unwrap();
    Ok(result)
}

/// Run two warp instances.
///
/// The first one contains needed routes that will be count in `/metrics` report.
/// The second serves `/metrics` and default liveness endpoints (livez/startz/readyz) without checker fns.
/// To setup checker on z-endpoint, don't add any filter to `routes`,
/// use instead `extra_liveness_routes` argument: `Some(livez().with_checker(...))`
///
/// `stats_port` is used to define custom port of second instance. Default value is `port` + `STATS_PORT_OFFSET`
pub async fn run_warp_with_stats<F, R>(
    routes: F,
    port: u16,
    extra_liveness_routes: Option<BoxedFilter<(HealthcheckReply,)>>,
    stats_port: Option<u16>,
) where
    R: Reply,
    F: Filter<Extract = (R,)> + Clone + Shared,
{
    register_metrics();

    let metrics_filter = warp::path!("metrics")
        .and(warp::get())
        .and_then(metrics_handler);
    let default_stats = metrics_filter.or(livez()).or(readyz()).or(startz());

    let stats = match extra_liveness_routes {
        Some(sr) => sr
            .or(default_stats)
            .map(|f| Box::new(f) as Box<dyn Reply>)
            .boxed(),
        None => default_stats.map(|f| Box::new(f) as Box<dyn Reply>).boxed(),
    };

    join(
        warp::serve(routes.with(warp::log::custom(estimate_request))).run(([0, 0, 0, 0], port)),
        warp::serve(stats).run(([0, 0, 0, 0], stats_port.unwrap_or(port + STATS_PORT_OFFSET))),
    )
    .await;
}
