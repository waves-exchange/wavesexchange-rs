use super::{
    liveness::{LivenessReply, Shared},
    livez, readyz, startz,
};
use futures::future::join;
use lazy_static::lazy_static;
use prometheus::{core::Collector, HistogramOpts, HistogramVec, IntCounter, Registry, TextEncoder};
use warp::{filters::BoxedFilter, log::Info, Filter, Rejection, Reply};

lazy_static! {
    static ref REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").unwrap();
    static ref RESPONSE_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("response_duration", "Response duration in secs"),
        &["code", "method"]
    )
    .unwrap();
}

pub const STATS_PORT_OFFSET: u16 = 1010;

pub trait SharedFilter<R>: Filter<Extract = (R,), Error = Rejection> + Clone + Shared {}

impl<F, R> SharedFilter<R> for F where F: Filter<Extract = (R,), Error = Rejection> + Clone + Shared {}

fn estimate_request(info: Info) {
    REQUESTS.inc();
    RESPONSE_DURATION
        .with_label_values(&[info.status().as_str(), info.method().as_str()])
        .observe(info.elapsed().as_secs_f64());
}

async fn metrics_handler(reg: Registry) -> impl Reply {
    TextEncoder::new().encode_to_string(&reg.gather()).unwrap()
}

/// Run two warp instances.
///
/// The first one contains needed routes that will be count in `/metrics` report.
/// The second serves `/metrics` and default liveness endpoints (livez/startz/readyz) without checker fns.
/// To setup checker on z-endpoint, don't add any filter to `routes`,
/// use instead `extra_liveness_routes` argument: `Some(livez().with_checker(...))`
///
/// `stats_port` is used to define custom port of second instance. Default value is `port` + `STATS_PORT_OFFSET`

type DeepBoxedFilter = BoxedFilter<(Box<dyn Reply>,)>;

pub struct StatsWarpBuilder {
    registry: Registry,
    main_routes: Option<DeepBoxedFilter>,
    stats_routes: DeepBoxedFilter,
    stats_port: Option<u16>,
}

impl Default for StatsWarpBuilder {
    fn default() -> Self {
        let registry = Registry::new();
        let warp_registry = registry.clone();
        let metrics_filter = warp::path!("metrics")
            .and(warp::get())
            .and(warp::any().map(move || warp_registry.clone()))
            .then(metrics_handler);
        let stats_routes = metrics_filter.or(livez()).or(readyz()).or(startz());
        Self {
            main_routes: None,
            stats_routes: deep_box_filter(stats_routes),
            stats_port: None,
            registry,
        }
    }
}

impl StatsWarpBuilder {
    pub fn from_routes<R, F>(routes: F) -> Self
    where
        R: Reply + 'static,
        F: SharedFilter<R>,
    {
        Self {
            main_routes: Some(deep_box_filter(routes)),
            ..Default::default()
        }
    }

    pub fn no_main_instance() -> Self {
        StatsWarpBuilder::default()
    }

    pub fn set_stats_port(mut self, port: u16) -> Self {
        self.stats_port = Some(port);
        self
    }

    pub fn override_liveness_routes(mut self, routes: impl SharedFilter<LivenessReply>) -> Self {
        self.stats_routes = deep_box_filter(routes.or(self.stats_routes));
        self
    }

    pub fn add_metric(self, metric: impl Collector + 'static) -> Self {
        self.registry.register(Box::new(metric)).unwrap();
        self
    }

    pub async fn run(mut self, port: u16) {
        self = self
            .add_metric(REQUESTS.clone())
            .add_metric(RESPONSE_DURATION.clone());

        let Self {
            main_routes,
            stats_port,
            stats_routes,
            ..
        } = self;

        let host = [0, 0, 0, 0];
        let warp_stats_instance =
            warp::serve(stats_routes).run((host, stats_port.unwrap_or(port + STATS_PORT_OFFSET)));

        match main_routes {
            Some(routes) => {
                join(
                    warp::serve(routes.with(warp::log::custom(estimate_request))).run((host, port)),
                    warp_stats_instance,
                )
                .await;
            }
            None => warp_stats_instance.await,
        }
    }
}

fn deep_box_filter<R, F>(filter: F) -> DeepBoxedFilter
where
    R: Reply + 'static,
    F: SharedFilter<R>,
{
    filter.map(|f| Box::new(f) as Box<dyn Reply>).boxed()
}
