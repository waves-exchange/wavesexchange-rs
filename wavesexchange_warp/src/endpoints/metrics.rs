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

/// A warp wrapper that provides liveness endpoints (`livez/startz/readyz`)
/// and extensible metrics collection for gathering requests (or any) stats.
/// Creates 1 or 2 warp instances.
///
/// The first instance serves `GET /metrics` route and
/// liveness endpoints (without checker fns on default). Mandatory.
///
/// The second one contains main provided routes that will be count in `/metrics` report.
/// If no routes provided, instance won't be created.
///
/// Example:
/// ```rust
/// let routes = warp::path!("hello").and(warp::get());
///
/// // run only stats instance on port 8080
/// StatsWarpBuilder::new().run_blocking(8080).await;
///
/// // run two warp instances on ports 8080 (main routes) and 9090 (stats routes)
/// // (default port for stats is main_routes_port + 1010),
/// // stats port can be overriden via `with_stats_port`
/// StatsWarpBuilder::new().with_main_routes(routes).run_blocking(8080).await;
/// ```
pub struct StatsWarpBuilder {
    registry: Registry,
    main_routes: Option<DeepBoxedFilter>,
    stats_routes: DeepBoxedFilter,
    stats_port: Option<u16>,
}

impl StatsWarpBuilder {
    /// Create and init builder with stats warp routes
    pub fn new() -> Self {
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

    /// Add routes for main warp instance
    ///
    /// Note: you shouldn't provide liveness endpoints in your routes, use `with_liveness_routes` method instead
    pub fn with_main_routes<R, E, F>(mut self, routes: F) -> Self
    where
        R: Reply + 'static,
        E: Into<Rejection>,
        F: SharedFilter<R, E>,
    {
        self.main_routes = Some(deep_box_filter(routes));
        self
    }

    /// Define custom port of stats instance.
    pub fn with_stats_port(mut self, port: u16) -> Self {
        self.stats_port = Some(port);
        self
    }

    /// Overwrite any liveness endpoint. Other liveness endpoints will not be affected.
    ///
    /// Example:
    /// ```rust
    /// use wavesexchange_warp::endpoints::{livez, Checkz};
    ///
    /// StatsWarpBuilder::new()
    ///     .with_liveness_routes(livez().with_checker(|| async { Ok(()) }))
    ///     .run_blocking()
    ///     .await;
    /// ```
    pub fn with_liveness_routes(mut self, routes: impl SharedFilter<LivenessReply>) -> Self {
        self.stats_routes = deep_box_filter(routes.or(self.stats_routes));
        self
    }

    /// Register prometheus metric. No need to `Box::new`.
    ///
    /// Note: if metric is created by `lazy_static!` or analogues, deref it first:
    /// ```rust
    /// .with_metric(&*MY_STATIC_METRIC)
    /// ```
    pub fn with_metric<M: Collector + Clone + 'static>(self, metric: &M) -> Self {
        self.registry.register(Box::new(metric.clone())).unwrap();
        self
    }

    /// Run warp instance(s) on current thread.
    ///
    /// Note: if running in a stats-only variant, `port` argument will be used for stats instance,
    /// otherwise it will be used by main instance,
    /// and stats will have `port + STATS_PORT_OFFSET` port
    /// (or custom if was set explicitly with `with_stats_port`)
    pub async fn run_blocking(mut self, port: u16) {
        self = self
            .with_metric(&*REQUESTS)
            .with_metric(&*RESPONSE_DURATION);

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
