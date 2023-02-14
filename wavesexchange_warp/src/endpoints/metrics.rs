use super::{
    liveness::{LivenessReply, Shared},
    livez as livez_fn, readyz as readyz_fn, startz as startz_fn, Checkz,
};
use futures::future::{join, BoxFuture, FutureExt};
use lazy_static::lazy_static;
use prometheus::{core::Collector, HistogramOpts, HistogramVec, IntCounter, Registry, TextEncoder};
use std::{env, fmt::Debug, future::Future};
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

pub const DEFAULT_MAIN_ROUTES_PORT: u16 = 8080;
pub const DEFAULT_METRICS_PORT_OFFSET: u16 = 1010;
pub const METRICS_PORT_ENV: &str = "METRICS_PORT";

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

pub fn reset_metrics() {
    REQUESTS.reset();
    RESPONSE_DURATION.reset();
}

async fn metrics_handler(reg: Registry) -> impl Reply {
    TextEncoder::new().encode_to_string(&reg.gather()).unwrap()
}

type DeepBoxedFilter<R = Box<dyn Reply>> = BoxedFilter<(R,)>;

/// A warp wrapper that provides liveness endpoints (`livez/startz/readyz`)
/// and extensible metrics collection for gathering requests (or any) statistics.
/// Creates 1 or 2 warp instances.
///
/// The first instance serves `GET /metrics` route and
/// liveness endpoints (without checker fns on default). Mandatory.
///
/// The second one contains main provided routes that will be count in `/metrics` report.
/// If no routes provided, instance won't be created.
///
/// Example:
/// ```no_run
/// # use std::convert::Infallible;
/// # use wavesexchange_warp::MetricsWarpBuilder;
/// # use warp::Filter;
/// # tokio_test::block_on(async {
/// let routes = warp::path!("hello").and_then(|| async { Ok::<_, Infallible>("Hello, world!") });
///
/// // run only metrics instance on port 8080
/// MetricsWarpBuilder::new().with_metrics_port(8080).run_async().await;
///
/// // run two warp instances on ports 8080 (main routes) and 9090 (metrics routes)
/// // (default port for metrics is main_routes_port + 1010),
/// // metrics port can be overridden via `with_metrics_port`
/// MetricsWarpBuilder::new().with_main_routes(routes).with_main_routes_port(8080).run_async().await;
///
/// // run only metrics instance on port defined in the METRICS_PORT env variable
/// MetricsWarpBuilder::new().with_metrics_port_from_env().run_async().await;
/// # })
/// ```
pub struct MetricsWarpBuilder {
    registry: Registry,
    main_routes: Option<DeepBoxedFilter>,
    main_routes_port: Option<u16>,
    metrics_port: Option<u16>,
    livez: DeepBoxedFilter<LivenessReply>,
    readyz: DeepBoxedFilter<LivenessReply>,
    startz: DeepBoxedFilter<LivenessReply>,
    graceful_shutdown_signal: Option<BoxFuture<'static, ()>>,
}

impl MetricsWarpBuilder {
    /// Create and init builder with metrics and liveness routes
    pub fn new() -> Self {
        Self {
            main_routes: None,
            main_routes_port: None,
            metrics_port: None,
            registry: Registry::new(),
            livez: livez_fn().boxed(),
            readyz: readyz_fn().boxed(),
            startz: startz_fn().boxed(),
            graceful_shutdown_signal: None,
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

    /// Define port number of main web-server instance.
    pub fn with_main_routes_port(mut self, port: u16) -> Self {
        self.main_routes_port = Some(port);
        self
    }

    /// Define port number of the metrics web-server instance.
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = Some(port);
        self
    }

    /// Use `METRICS_PORT` env variable as the port number of the metrics web-server instance, if set.
    /// If the env variable is not set, use default port number which is the main port number + 1010.
    pub fn with_metrics_port_from_env(mut self) -> Self {
        self.metrics_port = env::var(METRICS_PORT_ENV)
            .ok()
            .map(|s| s.parse::<u16>().ok())
            .flatten();
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
    /// ```no_run
    /// # use lazy_static::lazy_static;
    /// # use prometheus::IntCounter;
    /// # use wavesexchange_warp::MetricsWarpBuilder;
    /// # let builder = MetricsWarpBuilder::new();
    /// lazy_static! {
    ///     static ref MY_STATIC_METRIC: IntCounter = IntCounter::new("...", "...").unwrap();
    /// }
    /// builder.with_metric(&*MY_STATIC_METRIC);
    /// ```
    pub fn with_metric<M: Collector + Clone + 'static>(self, metric: &M) -> Self {
        self.registry.register(Box::new(metric.clone())).unwrap();
        self
    }

    pub fn with_graceful_shutdown<F>(mut self, signal: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.graceful_shutdown_signal = Some(Box::pin(signal));
        self
    }

    /// Run warp instance(s) on current thread
    pub async fn run_async(mut self) {
        self = self
            .with_metric(&*REQUESTS)
            .with_metric(&*RESPONSE_DURATION);

        let Self {
            main_routes,
            main_routes_port,
            metrics_port,
            registry,
            livez,
            readyz,
            startz,
            graceful_shutdown_signal,
        } = self;

        let host = [0, 0, 0, 0];
        let main_routes_port = main_routes_port.unwrap_or(DEFAULT_MAIN_ROUTES_PORT);
        let metrics_port = metrics_port.unwrap_or(main_routes_port + DEFAULT_METRICS_PORT_OFFSET);
        let metrics_filter = warp::path!("metrics")
            .and(warp::get())
            .and(warp::any().map(move || registry.clone()))
            .then(metrics_handler);

        let warp_metrics_instance_prepared =
            warp::serve(metrics_filter.or(livez).or(readyz).or(startz));

        match main_routes {
            Some(routes) => {
                let warp_main_instance_prepared =
                    warp::serve(routes.with(warp::log::custom(estimate_request)));

                match graceful_shutdown_signal {
                    Some(signal) => {
                        let shared_signal = signal.shared();
                        let (_addr, warp_main_instance) = warp_main_instance_prepared
                            .bind_with_graceful_shutdown(
                                (host, main_routes_port),
                                shared_signal.clone(),
                            );
                        let (_addr, warp_metrics_instance) = warp_metrics_instance_prepared
                            .bind_with_graceful_shutdown((host, metrics_port), shared_signal);
                        join(warp_main_instance, warp_metrics_instance).await;
                    }
                    None => {
                        let warp_main_instance =
                            warp_main_instance_prepared.run((host, main_routes_port));
                        let warp_metrics_instance =
                            warp_metrics_instance_prepared.run((host, metrics_port));
                        join(warp_main_instance, warp_metrics_instance).await;
                    }
                }
            }
            None => match graceful_shutdown_signal {
                Some(signal) => {
                    let (_addr, warp_metrics_instance) = warp_metrics_instance_prepared
                        .bind_with_graceful_shutdown((host, metrics_port), signal);
                    warp_metrics_instance.await;
                }
                None => {
                    warp_metrics_instance_prepared
                        .run((host, metrics_port))
                        .await
                }
            },
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
