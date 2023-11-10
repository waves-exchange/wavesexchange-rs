use super::liveness::{
    livez as livez_fn, readyz as readyz_fn, startz as startz_fn, Checkz, LivenessReply, Readiness,
    Shared,
};
use futures::future::{join, BoxFuture, FutureExt};
use lazy_static::lazy_static;
use prometheus::{core::Collector, HistogramOpts, HistogramVec, IntCounter, Registry, TextEncoder};
use std::{
    env,
    fmt::Debug,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, oneshot},
    task,
};
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

    /// Provide a oneshot channel for 'initialization finished' signal,
    /// once it is received the service will start to report that it is ready.
    ///
    /// Example:
    /// ```no_run
    /// use tokio::sync::oneshot;
    /// # use wavesexchange_warp::MetricsWarpBuilder;
    /// # let builder = MetricsWarpBuilder::new();
    /// let (tx, rx) = oneshot::channel();
    /// let server_future = builder.with_init_channel(rx);
    /// // ... run initialization code ...
    /// tx.send(()).unwrap();
    /// ```
    pub fn with_init_channel(mut self, chn: oneshot::Receiver<()>) -> Self {
        let is_initialized = Arc::new(Mutex::new(false));

        task::spawn({
            let is_initialized = is_initialized.clone();
            async move {
                match chn.await {
                    Ok(()) => {
                        let mut is_initialized = is_initialized.lock().unwrap();
                        *is_initialized = true;
                    }
                    Err(_) => {
                        // Sender was dropped before sending a message,
                        // which means something went wrong and initialization
                        // will never succeed, so we panic here
                        panic!("initialization failed?");
                    }
                }
            }
        });

        self.readyz = readyz_fn()
            .with_checker(move || async move {
                let is_initialized = is_initialized.lock().unwrap();
                if *is_initialized {
                    Ok(())
                } else {
                    Err(ServiceStatusError::InitInProgress)
                }
            })
            .boxed();

        self
    }

    /// Provide a channel for readiness status changes.
    ///
    /// Example:
    /// ```no_run
    /// use tokio::sync::mpsc;
    /// use wavesexchange_warp::endpoints::Readiness;
    /// # use wavesexchange_warp::MetricsWarpBuilder;
    /// # let builder = MetricsWarpBuilder::new();
    /// let (tx, rx) = mpsc::unbounded_channel();
    /// let server_future = builder.with_readiness_channel(rx);
    /// // ... default status is Ready ...
    /// tx.send(Readiness::NotReady).unwrap(); // Something bad happened
    /// // . . . . .
    /// tx.send(Readiness::Ready).unwrap(); // Things are back to normal
    /// // . . . . .
    /// tx.send(Readiness::Dead).unwrap(); // Something's screwed up, service will be killed by the orchestration framework
    /// ```
    pub fn with_readiness_channel(mut self, mut chn: mpsc::UnboundedReceiver<Readiness>) -> Self {
        let readiness = Arc::new(Mutex::new(Readiness::Ready));

        task::spawn({
            let readiness = readiness.clone();
            async move {
                while let Some(status) = chn.recv().await {
                    let mut readiness = readiness.lock().unwrap();
                    *readiness = status;
                }
                // All senders were dropped, so no new messages can ever be received,
                // and the current readiness status is final.
                // If it indicates "not ready" - we panic, because anyway it could
                // not be changed back to "ready" anymore.
                let readiness = readiness.lock().unwrap();
                let final_state = *readiness;
                drop(readiness);
                if final_state != Readiness::Ready {
                    panic!("service will never be ready again - aborting");
                }
            }
        });

        self.readyz = readyz_fn()
            .with_checker({
                let readiness = readiness.clone();
                move || async move {
                    let readiness = readiness.lock().unwrap();
                    if *readiness == Readiness::Ready {
                        Ok(())
                    } else {
                        Err(ServiceStatusError::ServiceNotReady)
                    }
                }
            })
            .boxed();

        self.livez = livez_fn()
            .with_checker({
                let readiness = readiness.clone();
                move || async move {
                    let readiness = readiness.lock().unwrap();
                    if *readiness != Readiness::Dead {
                        Ok(())
                    } else {
                        Err(ServiceStatusError::ServiceDead)
                    }
                }
            })
            .boxed();

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

    /// Build Warp instance(s) and run them forever.
    /// If there is only one (metrics) Warp server instance, it will be run on the current Tokio task.
    /// In case of two Warp instances (main + metrics), one of them will be run on the current task,
    /// and the other on a separate task, to avoid any interference between them
    /// (e.g. programming errors in web handlers in main server will not affect the metrics server).
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

        let metrics_web_server = warp::serve(metrics_filter.or(livez).or(readyz).or(startz));

        match main_routes {
            Some(routes) => {
                let main_web_server = warp::serve(routes.with(warp::log::custom(estimate_request)));

                let (main_server, metrics_server) = match graceful_shutdown_signal {
                    Some(signal) => {
                        let signal = signal.shared();
                        let (_addr, main_server) = main_web_server
                            .bind_with_graceful_shutdown((host, main_routes_port), signal.clone());
                        let (_addr, metrics_server) = metrics_web_server
                            .bind_with_graceful_shutdown((host, metrics_port), signal);
                        (main_server.boxed(), metrics_server.boxed())
                    }
                    None => {
                        let main_server = main_web_server.run((host, main_routes_port));
                        let metrics_server = metrics_web_server.run((host, metrics_port));
                        (main_server.boxed(), metrics_server.boxed())
                    }
                };
                // Run both web-servers on different Tokio tasks to avoid any unanticipated interference
                let metrics_server = task::spawn(metrics_server);
                let ((), task_err) = join(main_server, metrics_server).await;
                task_err.expect("metrics web-server panicked");
            }
            None => match graceful_shutdown_signal {
                Some(signal) => {
                    let (_addr, metrics_server) = metrics_web_server
                        .bind_with_graceful_shutdown((host, metrics_port), signal);
                    metrics_server.await;
                }
                None => metrics_web_server.run((host, metrics_port)).await,
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

#[derive(Clone, Copy, thiserror::Error)]
enum ServiceStatusError {
    #[error("service initialization in progress")]
    InitInProgress,

    #[error("service not ready")]
    ServiceNotReady,

    #[error("service is dead")]
    ServiceDead,
}

impl Debug for ServiceStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}
