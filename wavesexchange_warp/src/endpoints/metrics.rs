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

pub trait SharedFilter<R, E: Into<Rejection> = Rejection>:
    Filter<Extract = (R,), Error = E> + Clone + Shared
{
}

impl<R, E, F> SharedFilter<R, E> for F
where
    E: Into<Rejection>,
    F: Filter<Extract = (R,), Error = E> + Clone + Shared,
{
}

fn estimate_request(info: Info) {
    REQUESTS.inc();
    RESPONSE_DURATION
        .with_label_values(&[info.status().as_str(), info.method().as_str()])
        .observe(info.elapsed().as_secs_f64());
}

async fn metrics_handler(reg: Registry) -> impl Reply {
    TextEncoder::new().encode_to_string(&reg.gather()).unwrap()
}

type DeepBoxedFilter = BoxedFilter<(Box<dyn Reply>,)>;

/// Instanciate the given optional warp instance and an extra listener that serves stats: liveness endpoints and metrics.
///
/// The first instance contains needed routes that will be count in `/metrics` report.
/// Can be skipped with `StatsWarpBuilder::no_main_instance` if only stats required.
/// The second serves `/metrics` and default liveness endpoints (livez/startz/readyz) without checker fns. Mandatory.
///
/// Example:
/// ```rust
/// let routes = warp::path!("hello").and(warp::get());
///
/// // run two warp instances on ports 8080 and 9090 (default port offset for stats is 1010),
/// // stats port can be overriden via `set_stats_port`
/// StatsWarpBuilder::from_routes(routes).run(8080).await;
///
/// // run one stats instance on port 8080
/// StatsWarpBuilder::no_main_instance().run(8080).await;
/// ```
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
    /// Create two warp instances: for given routes and for stats.  
    ///
    /// Note: you shouldn't provide overrides for liveness endpoints here, use `override_liveness_routes` method instead
    pub fn from_routes<R, E, F>(routes: F) -> Self
    where
        R: Reply + 'static,
        E: Into<Rejection>,
        F: SharedFilter<R, E>,
    {
        Self {
            main_routes: Some(deep_box_filter(routes)),
            ..Default::default()
        }
    }

    /// Create only one warp stats instance
    pub fn no_main_instance() -> Self {
        StatsWarpBuilder::default()
    }

    /// Define custom port of stats instance.
    /// Default value is `port` + `STATS_PORT_OFFSET` if main instance is present
    /// and `port` if not
    pub fn set_stats_port(mut self, port: u16) -> Self {
        self.stats_port = Some(port);
        self
    }

    /// Add checker function to any liveness endpoint
    ///
    /// Example:
    /// ```rust
    /// use wavesexchange_warp::endpoints::{livez, Checkz};
    ///
    /// StatsWarpBuilder::no_main_instance()
    ///     .override_liveness_routes(livez().with_checker(|| async { Ok(()) }))
    ///     .run
    ///     .await;
    /// ```
    pub fn override_liveness_routes(mut self, routes: impl SharedFilter<LivenessReply>) -> Self {
        self.stats_routes = deep_box_filter(routes.or(self.stats_routes));
        self
    }

    /// Register prometheus metric. No need to `Box::new`.
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
        let stats_port = stats_port.unwrap_or(if main_routes.is_some() {
            port + STATS_PORT_OFFSET
        } else {
            port
        });
        let warp_stats_instance = warp::serve(stats_routes).run((host, stats_port));

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

fn deep_box_filter<R, E, F>(filter: F) -> DeepBoxedFilter
where
    R: Reply + 'static,
    E: Into<Rejection>,
    F: SharedFilter<R, E>,
{
    filter.map(|f| Box::new(f) as Box<dyn Reply>).boxed()
}
