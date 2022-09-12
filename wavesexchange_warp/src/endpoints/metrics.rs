use super::{
    liveness::{LivenessReply, Shared},
    livez as livez_fn, readyz as readyz_fn, startz as startz_fn, Checkz,
};
use futures::future::join;
use lazy_static::lazy_static;
use prometheus::{core::Collector, HistogramOpts, HistogramVec, IntCounter, Registry, TextEncoder};
use std::{fmt::Debug, future::Future};
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

type DeepBoxedFilter<R = Box<dyn Reply>> = BoxedFilter<(R,)>;

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
    stats_port: Option<u16>,
    livez: DeepBoxedFilter<LivenessReply>,
    readyz: DeepBoxedFilter<LivenessReply>,
    startz: DeepBoxedFilter<LivenessReply>,
}

impl StatsWarpBuilder {
    /// Create and init builder with stats warp routes
    pub fn new() -> Self {
        Self {
            main_routes: None,
            stats_port: None,
            registry: Registry::new(),
            livez: livez_fn().boxed(),
            readyz: readyz_fn().boxed(),
            startz: startz_fn().boxed(),
        }
    }

    /// Add routes for main warp instance
    ///
    /// Note: you shouldn't provide liveness endpoints in your routes, use `with_*z_checker` methods instead
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

    pub fn with_livez_checker<F, C, E>(mut self, checker: C) -> Self
    where
        E: Debug + Shared,
        F: Future<Output = Result<(), E>> + Send,
        C: FnOnce() -> F + Clone + Shared,
    {
        self.livez = livez_fn().with_checker(checker).boxed();
        self
    }

    pub fn with_readyz_checker<F, C, E>(mut self, checker: C) -> Self
    where
        E: Debug + Shared,
        F: Future<Output = Result<(), E>> + Send,
        C: FnOnce() -> F + Clone + Shared,
    {
        self.readyz = readyz_fn().with_checker(checker).boxed();
        self
    }

    pub fn with_startz_checker<F, C, E>(mut self, checker: C) -> Self
    where
        E: Debug + Shared,
        F: Future<Output = Result<(), E>> + Send,
        C: FnOnce() -> F + Clone + Shared,
    {
        self.startz = startz_fn().with_checker(checker).boxed();
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
            registry,
            livez,
            readyz,
            startz,
        } = self;

        let host = [0, 0, 0, 0];
        let stats_port = stats_port.unwrap_or(if main_routes.is_some() {
            port + STATS_PORT_OFFSET
        } else {
            port
        });
        let metrics_filter = warp::path!("metrics")
            .and(warp::get())
            .and(warp::any().map(move || registry.clone()))
            .then(metrics_handler);
        let warp_stats_instance =
            warp::serve(metrics_filter.or(livez).or(readyz).or(startz)).run((host, stats_port));

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
