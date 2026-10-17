use prometheus::{
    CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
    TextEncoder,
};
use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Rolling windows reported by [`Metrics::set_prediction_window`].
///
/// Each entry is `(label, window length in seconds)`.
pub const PREDICTION_WINDOWS: [(&str, i64); 4] =
    [("1m", 60), ("5m", 300), ("1h", 3_600), ("24h", 86_400)];

/// Shared application metrics exposed to Prometheus.
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,

    // ── HTTP ─────────────────────────────────────────────────────────────────
    pub http_requests_total: CounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_server_errors_total: prometheus::Counter,

    // ── Price cache ───────────────────────────────────────────────────────────
    pub price_cache_fetch_total: CounterVec,
    pub price_cache_assets: Gauge,
    pub price_cache_fetch_duration_seconds: Histogram,

    // ── Application / runtime ────────────────────────────────────────────────
    pub app_up: Gauge,
    pub app_info: Gauge,
    pub memory_used_bytes: Gauge,
    pub memory_total_bytes: Gauge,

    // ── Database query durations ─────────────────────────────────────────────
    /// Histogram of DB query durations labelled by `query_type`
    /// (e.g. "get_pool", "list_pools", "insert_prediction", …).
    pub db_query_duration_seconds: HistogramVec,
    /// Total DB queries by query type and result (ok / error).
    pub db_queries_total: CounterVec,

    // ── Redis operation latencies ─────────────────────────────────────────────
    /// Histogram of Redis operation durations labelled by `operation`
    /// (e.g. "get", "set", "del", "ping", "hset", …).
    pub redis_operation_duration_seconds: HistogramVec,
    /// Total Redis operations by operation type and result.
    pub redis_operations_total: CounterVec,
    /// Total Redis cache lookups by cache name and outcome
    /// (`"hit"` / `"miss"` / `"error"`), from [`crate::redis_cache::RedisCache`].
    /// A per-cache hit ratio can be computed as
    /// `hit / (hit + miss)` grouped by `cache_name`.
    pub redis_cache_lookups_total: CounterVec,

    // ── WebSocket ─────────────────────────────────────────────────────────────
    /// Current number of open WebSocket connections.
    pub ws_connections_active: Gauge,
    /// Total WebSocket connections accepted since startup.
    pub ws_connections_total: prometheus::Counter,
    /// Total WebSocket connections closed since startup.
    pub ws_disconnections_total: prometheus::Counter,
    /// WebSocket frames sent to clients, labelled by `kind`
    /// (`"event"`, `"batch"`, `"control"`) and `encoding` (`"text"` / `"deflate"`).
    pub ws_messages_sent_total: CounterVec,
    /// Bytes written to WebSocket clients (after compression), by `encoding`.
    pub ws_bytes_sent_total: CounterVec,
    /// Events that were not delivered, labelled by `reason`
    /// (`"lagged"`, `"oversize"`, `"filtered"`).
    pub ws_events_dropped_total: CounterVec,
    /// Number of events packed into each outbound batch frame.
    pub ws_batch_size: Histogram,

    // ── Pool state gauges ────────────────────────────────────────────────────
    /// Currently active (open) prediction pools.
    pub active_pools: Gauge,
    /// Currently resolved prediction pools.
    pub resolved_pools: Gauge,
    /// Currently cancelled prediction pools.
    pub cancelled_pools: Gauge,

    // ── Prediction volume ─────────────────────────────────────────────────────
    /// Monotonic count of predictions indexed by this process. Use
    /// `increase(app_predictions_total[<window>])` for arbitrary windows.
    pub predictions_total: prometheus::Counter,
    /// Monotonic sum of prediction stakes (stroops) indexed by this process.
    pub prediction_volume_stroops_total: prometheus::Counter,
    /// Predictions placed in the trailing `window` (`1m`/`5m`/`1h`/`24h`),
    /// computed from the database by [`run_collector_loop`] so it is correct
    /// across restarts and multiple replicas.
    pub predictions_window: GaugeVec,
    /// Stake volume (stroops) placed in the trailing `window`.
    pub prediction_volume_window_stroops: GaugeVec,

    // ── Worker queue ─────────────────────────────────────────────────────────
    /// Current number of jobs waiting or retrying in the worker queue.
    pub worker_queue_depth: Gauge,
    /// Current number of jobs in the worker dead-letter queue (DLQ).
    pub worker_dlq_depth: Gauge,
}

/// Type alias for a reference-counted [`Metrics`] instance shared across handlers.
pub type SharedMetrics = Arc<Metrics>;

impl Metrics {
    /// Create and register all Prometheus metrics with a fresh [`Registry`].
    ///
    /// Returns an error if any metric fails to register (e.g. duplicate name).
    /// In practice this should never fail because the metric names are
    /// hard-coded constants.
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // ── HTTP ──────────────────────────────────────────────────────────────
        let http_requests_total = CounterVec::new(
            Opts::new(
                "app_http_requests_total",
                "Total number of HTTP requests served by the backend.",
            ),
            &["method", "path", "status"],
        )?;

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "app_http_request_duration_seconds",
                "HTTP request latency in seconds.",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "path", "status"],
        )?;

        let http_server_errors_total = prometheus::Counter::with_opts(Opts::new(
            "app_http_500_errors_total",
            "Total number of HTTP 500 (Internal Server Error) responses.",
        ))?;

        // ── Price cache ───────────────────────────────────────────────────────
        let price_cache_fetch_total = CounterVec::new(
            Opts::new(
                "app_price_cache_fetch_total",
                "Total CoinGecko price-cache refresh attempts.",
            ),
            &["result"],
        )?;

        let price_cache_assets = Gauge::with_opts(Opts::new(
            "app_price_cache_assets",
            "Number of assets currently stored in the price cache.",
        ))?;

        let price_cache_fetch_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "app_price_cache_fetch_duration_seconds",
                "CoinGecko price-cache refresh latency in seconds.",
            )
            .buckets(vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]),
        )?;

        // ── Application / runtime ─────────────────────────────────────────────
        let app_up = Gauge::with_opts(Opts::new("app_up", "Application availability status."))?;
        app_up.set(1.0);

        let app_info = Gauge::with_opts(
            Opts::new("app_build_info", "Static metadata about the backend build.")
                .const_label("service", "predifi-backend")
                .const_label("version", env!("CARGO_PKG_VERSION")),
        )?;
        app_info.set(1.0);

        let memory_used_bytes = Gauge::with_opts(Opts::new(
            "app_memory_used_bytes",
            "Memory used by the backend in bytes.",
        ))?;
        let memory_total_bytes = Gauge::with_opts(Opts::new(
            "app_memory_total_bytes",
            "Total system memory in bytes.",
        ))?;

        // ── Database query durations ──────────────────────────────────────────
        let db_query_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "app_db_query_duration_seconds",
                "Database query latency in seconds, labelled by query type.",
            )
            .buckets(vec![
                0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0,
            ]),
            &["query_type"],
        )?;

        let db_queries_total = CounterVec::new(
            Opts::new(
                "app_db_queries_total",
                "Total database queries by query type and result.",
            ),
            &["query_type", "result"],
        )?;

        // ── Redis operation latencies ─────────────────────────────────────────
        let redis_operation_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "app_redis_operation_duration_seconds",
                "Redis operation latency in seconds, labelled by operation.",
            )
            .buckets(vec![
                0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.5,
            ]),
            &["operation"],
        )?;

        let redis_operations_total = CounterVec::new(
            Opts::new(
                "app_redis_operations_total",
                "Total Redis operations by operation type and result.",
            ),
            &["operation", "result"],
        )?;

        let redis_cache_lookups_total = CounterVec::new(
            Opts::new(
                "app_redis_cache_lookups_total",
                "Total Redis cache lookups by cache name and outcome (hit/miss/error). \
                 A hit ratio can be computed as hit / (hit + miss), grouped by cache_name.",
            ),
            &["cache_name", "outcome"],
        )?;

        // ── WebSocket ─────────────────────────────────────────────────────────
        let ws_connections_active = Gauge::with_opts(Opts::new(
            "app_ws_connections_active",
            "Current number of open WebSocket connections.",
        ))?;

        let ws_connections_total = prometheus::Counter::with_opts(Opts::new(
            "app_ws_connections_total",
            "Total WebSocket connections accepted since startup.",
        ))?;

        let ws_disconnections_total = prometheus::Counter::with_opts(Opts::new(
            "app_ws_disconnections_total",
            "Total WebSocket connections closed since startup.",
        ))?;

        let ws_messages_sent_total = CounterVec::new(
            Opts::new(
                "app_ws_messages_sent_total",
                "WebSocket frames sent to clients by kind and encoding.",
            ),
            &["kind", "encoding"],
        )?;

        let ws_bytes_sent_total = CounterVec::new(
            Opts::new(
                "app_ws_bytes_sent_total",
                "Bytes written to WebSocket clients (after compression) by encoding.",
            ),
            &["encoding"],
        )?;

        let ws_events_dropped_total = CounterVec::new(
            Opts::new(
                "app_ws_events_dropped_total",
                "WebSocket events not delivered to a client, by reason.",
            ),
            &["reason"],
        )?;

        let ws_batch_size = Histogram::with_opts(
            HistogramOpts::new(
                "app_ws_batch_size",
                "Number of events packed into each outbound WebSocket batch frame.",
            )
            .buckets(vec![1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0]),
        )?;

        // ── Pool state gauges ─────────────────────────────────────────────────
        let active_pools = Gauge::with_opts(Opts::new(
            "app_active_pools",
            "Number of currently active prediction market pools.",
        ))?;

        let resolved_pools = Gauge::with_opts(Opts::new(
            "app_resolved_pools",
            "Number of prediction pools whose outcome has been resolved.",
        ))?;

        let cancelled_pools = Gauge::with_opts(Opts::new(
            "app_cancelled_pools",
            "Number of prediction pools that have been cancelled.",
        ))?;

        // ── Prediction volume ─────────────────────────────────────────────────
        let predictions_total = prometheus::Counter::with_opts(Opts::new(
            "app_predictions_total",
            "Total number of predictions indexed by this process.",
        ))?;

        let prediction_volume_stroops_total = prometheus::Counter::with_opts(Opts::new(
            "app_prediction_volume_stroops_total",
            "Total prediction stake volume in stroops indexed by this process.",
        ))?;

        let predictions_window = GaugeVec::new(
            Opts::new(
                "app_predictions_window",
                "Predictions placed in the trailing time window (from the database).",
            ),
            &["window"],
        )?;

        let prediction_volume_window_stroops = GaugeVec::new(
            Opts::new(
                "app_prediction_volume_window_stroops",
                "Prediction stake volume in stroops placed in the trailing time window.",
            ),
            &["window"],
        )?;

        // ── Worker queue ──────────────────────────────────────────────────────
        let worker_queue_depth = Gauge::with_opts(Opts::new(
            "app_worker_queue_depth",
            "Current number of pending or retrying jobs in the worker queue.",
        ))?;

        let worker_dlq_depth = Gauge::with_opts(Opts::new(
            "app_worker_dlq_depth",
            "Current number of jobs in the worker dead-letter queue (DLQ).",
        ))?;

        // ── Register all metrics ──────────────────────────────────────────────
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;
        registry.register(Box::new(http_server_errors_total.clone()))?;
        registry.register(Box::new(price_cache_fetch_total.clone()))?;
        registry.register(Box::new(price_cache_assets.clone()))?;
        registry.register(Box::new(price_cache_fetch_duration_seconds.clone()))?;
        registry.register(Box::new(app_up.clone()))?;
        registry.register(Box::new(app_info.clone()))?;
        registry.register(Box::new(memory_used_bytes.clone()))?;
        registry.register(Box::new(memory_total_bytes.clone()))?;
        registry.register(Box::new(db_query_duration_seconds.clone()))?;
        registry.register(Box::new(db_queries_total.clone()))?;
        registry.register(Box::new(redis_operation_duration_seconds.clone()))?;
        registry.register(Box::new(redis_operations_total.clone()))?;
        registry.register(Box::new(redis_cache_lookups_total.clone()))?;
        registry.register(Box::new(ws_connections_active.clone()))?;
        registry.register(Box::new(ws_connections_total.clone()))?;
        registry.register(Box::new(ws_disconnections_total.clone()))?;
        registry.register(Box::new(ws_messages_sent_total.clone()))?;
        registry.register(Box::new(ws_bytes_sent_total.clone()))?;
        registry.register(Box::new(ws_events_dropped_total.clone()))?;
        registry.register(Box::new(ws_batch_size.clone()))?;
        registry.register(Box::new(active_pools.clone()))?;
        registry.register(Box::new(resolved_pools.clone()))?;
        registry.register(Box::new(cancelled_pools.clone()))?;
        registry.register(Box::new(predictions_total.clone()))?;
        registry.register(Box::new(prediction_volume_stroops_total.clone()))?;
        registry.register(Box::new(predictions_window.clone()))?;
        registry.register(Box::new(prediction_volume_window_stroops.clone()))?;
        registry.register(Box::new(worker_queue_depth.clone()))?;
        registry.register(Box::new(worker_dlq_depth.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_server_errors_total,
            price_cache_fetch_total,
            price_cache_assets,
            price_cache_fetch_duration_seconds,
            app_up,
            app_info,
            memory_used_bytes,
            memory_total_bytes,
            db_query_duration_seconds,
            db_queries_total,
            redis_operation_duration_seconds,
            redis_operations_total,
            redis_cache_lookups_total,
            ws_connections_active,
            ws_connections_total,
            ws_disconnections_total,
            ws_messages_sent_total,
            ws_bytes_sent_total,
            ws_events_dropped_total,
            ws_batch_size,
            active_pools,
            resolved_pools,
            cancelled_pools,
            predictions_total,
            prediction_volume_stroops_total,
            predictions_window,
            prediction_volume_window_stroops,
            worker_queue_depth,
            worker_dlq_depth,
        })
    }

    // ── Domain helpers ────────────────────────────────────────────────────────

    /// Record worker queue depth metrics.
    pub fn record_worker_queue_depth(&self, queue_depth: usize, dlq_depth: usize) {
        self.worker_queue_depth.set(queue_depth as f64);
        self.worker_dlq_depth.set(dlq_depth as f64);
    }

    /// Record the outcome of a price-cache refresh attempt.
    pub fn record_price_cache_fetch(&self, result: &str, assets: usize, duration_secs: f64) {
        self.price_cache_fetch_total
            .with_label_values(&[result])
            .inc();
        self.price_cache_assets.set(assets as f64);
        self.price_cache_fetch_duration_seconds
            .observe(duration_secs);
    }

    /// Record a database query's duration and result.
    ///
    /// `query_type` should be a short stable label such as `"get_pool"` or
    /// `"insert_prediction"`.  `result` should be `"ok"` or `"error"`.
    pub fn record_db_query(&self, query_type: &str, result: &str, duration_secs: f64) {
        self.db_query_duration_seconds
            .with_label_values(&[query_type])
            .observe(duration_secs);
        self.db_queries_total
            .with_label_values(&[query_type, result])
            .inc();
    }

    /// Record a Redis operation's duration and result.
    ///
    /// `operation` should be a short stable label such as `"get"`, `"set"`,
    /// `"del"`, or `"hset"`.  `result` should be `"ok"` or `"error"`.
    pub fn record_redis_op(&self, operation: &str, result: &str, duration_secs: f64) {
        self.redis_operation_duration_seconds
            .with_label_values(&[operation])
            .observe(duration_secs);
        self.redis_operations_total
            .with_label_values(&[operation, result])
            .inc();
    }

    /// Record a Redis cache lookup outcome for a named cache.
    ///
    /// `cache_name` should be a short stable label such as `"pools"`,
    /// `"pool"`, `"user"`, or `"stats"` (see
    /// [`crate::redis_cache::cache_name_from_key`]). `outcome` should be one
    /// of `"hit"`, `"miss"`, or `"error"`.
    pub fn record_cache_lookup(&self, cache_name: &str, outcome: &str) {
        self.redis_cache_lookups_total
            .with_label_values(&[cache_name, outcome])
            .inc();
    }

    /// Increment the active WebSocket connection count (call on connect).
    pub fn ws_connect(&self) {
        self.ws_connections_active.inc();
        self.ws_connections_total.inc();
    }

    /// Decrement the active WebSocket connection count (call on disconnect).
    pub fn ws_disconnect(&self) {
        self.ws_connections_active.dec();
        self.ws_disconnections_total.inc();
    }

    /// Set the pool state gauges.
    ///
    /// Pass the current counts of active, resolved, and cancelled pools as
    /// retrieved from the database or in-memory cache.
    pub fn set_pool_counts(&self, active: u64, resolved: u64, cancelled: u64) {
        self.active_pools.set(active as f64);
        self.resolved_pools.set(resolved as f64);
        self.cancelled_pools.set(cancelled as f64);
    }

    /// Record a new prediction event.
    ///
    /// Increments the monotonic prediction counters. Per-window totals are
    /// published separately by [`Metrics::set_prediction_window`].
    /// `amount_stroops` is the stake amount in Stellar stroops.
    pub fn record_prediction(&self, amount_stroops: u64) {
        self.predictions_total.inc();
        self.prediction_volume_stroops_total
            .inc_by(amount_stroops as f64);
    }

    /// Publish the prediction count and stake volume for one trailing window
    /// (one of the labels in [`PREDICTION_WINDOWS`]).
    pub fn set_prediction_window(&self, window: &str, count: u64, volume_stroops: u64) {
        self.predictions_window
            .with_label_values(&[window])
            .set(count as f64);
        self.prediction_volume_window_stroops
            .with_label_values(&[window])
            .set(volume_stroops as f64);
    }

    /// Record one outbound WebSocket frame.
    ///
    /// `kind` is `"event"`, `"batch"`, or `"control"`; `encoding` is
    /// `"text"` or `"deflate"`.
    pub fn record_ws_frame(&self, kind: &str, encoding: &str, bytes: usize) {
        self.ws_messages_sent_total
            .with_label_values(&[kind, encoding])
            .inc();
        self.ws_bytes_sent_total
            .with_label_values(&[encoding])
            .inc_by(bytes as f64);
    }

    /// Record `count` WebSocket events dropped for `reason`.
    pub fn record_ws_dropped(&self, reason: &str, count: u64) {
        self.ws_events_dropped_total
            .with_label_values(&[reason])
            .inc_by(count as f64);
    }

    /// Encode all registered metrics into the Prometheus text exposition format.
    ///
    /// Returns the UTF-8 encoded text ready to be served at `/metrics`.
    /// Returns an error if encoding fails or the output is not valid UTF-8
    /// (neither should happen in practice).
    pub fn gather_text(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| prometheus::Error::Msg(format!("failed to encode metrics: {e}")))?;
        String::from_utf8(buffer)
            .map_err(|e| prometheus::Error::Msg(format!("invalid metrics UTF-8: {e}")))
    }
}

// ── Process-wide metrics sink ─────────────────────────────────────────────────

static GLOBAL_METRICS: OnceLock<SharedMetrics> = OnceLock::new();

/// Install the process-wide metrics instance used by code paths that have no
/// handle to [`SharedMetrics`] (the `db` repository functions in particular).
///
/// Only the first call wins; later calls are ignored so tests that build many
/// routers cannot clobber the server's registry.
pub fn install_global(metrics: SharedMetrics) {
    let _ = GLOBAL_METRICS.set(metrics);
}

/// The metrics instance installed by [`install_global`], if any.
pub fn global() -> Option<&'static SharedMetrics> {
    GLOBAL_METRICS.get()
}

/// Await a database future and record its latency under `query_type` in
/// `app_db_query_duration_seconds` / `app_db_queries_total`.
///
/// A no-op wrapper when no global metrics instance is installed.
pub async fn observe_db<F, T, E>(query_type: &'static str, fut: F) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
{
    let start = Instant::now();
    let result = fut.await;
    if let Some(metrics) = global() {
        let outcome = if result.is_ok() { "ok" } else { "error" };
        metrics.record_db_query(query_type, outcome, start.elapsed().as_secs_f64());
    }
    result
}

// ── Background collector ──────────────────────────────────────────────────────

/// Map a `pools.state` value to the gauge it feeds.
///
/// `settled` pools are resolved; `closed` is what
/// [`crate::db::cancel_pool_in_db`] writes for an on-chain cancellation.
fn pool_state_bucket(state: &str) -> Option<&'static str> {
    match state {
        "active" => Some("active"),
        "settled" => Some("resolved"),
        "closed" => Some("cancelled"),
        _ => None,
    }
}

/// Refresh the database-derived gauges once: pool counts by state and
/// prediction count/volume per trailing window.
pub async fn collect_db_gauges(pool: &sqlx::PgPool, metrics: &Metrics) -> Result<(), sqlx::Error> {
    let rows: Vec<(String, i64)> = observe_db(
        "metrics_pool_counts",
        sqlx::query_as("SELECT state, COUNT(*) FROM pools GROUP BY state").fetch_all(pool),
    )
    .await?;

    let (mut active, mut resolved, mut cancelled) = (0u64, 0u64, 0u64);
    for (state, count) in rows {
        let count = count.max(0) as u64;
        match pool_state_bucket(&state) {
            Some("active") => active += count,
            Some("resolved") => resolved += count,
            Some("cancelled") => cancelled += count,
            _ => {}
        }
    }
    metrics.set_pool_counts(active, resolved, cancelled);

    // One pass over the last 24h of predictions (served by the
    // created_at indexes) yields every window at once.
    let (c1m, v1m, c5m, v5m, c1h, v1h, c24h, v24h): (i64, i64, i64, i64, i64, i64, i64, i64) =
        observe_db(
            "metrics_prediction_windows",
            sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '1 minute'),
                    COALESCE(SUM(amount) FILTER (WHERE created_at > NOW() - INTERVAL '1 minute'), 0)::BIGINT,
                    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '5 minutes'),
                    COALESCE(SUM(amount) FILTER (WHERE created_at > NOW() - INTERVAL '5 minutes'), 0)::BIGINT,
                    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '1 hour'),
                    COALESCE(SUM(amount) FILTER (WHERE created_at > NOW() - INTERVAL '1 hour'), 0)::BIGINT,
                    COUNT(*),
                    COALESCE(SUM(amount), 0)::BIGINT
                FROM predictions
                WHERE created_at > NOW() - INTERVAL '24 hours'
                "#,
            )
            .fetch_one(pool),
        )
        .await?;

    let values = [(c1m, v1m), (c5m, v5m), (c1h, v1h), (c24h, v24h)];
    for ((window, _), (count, volume)) in PREDICTION_WINDOWS.iter().zip(values) {
        metrics.set_prediction_window(window, count.max(0) as u64, volume.max(0) as u64);
    }
    Ok(())
}

/// Refresh process-level gauges that are cheap to sample: WebSocket
/// connections, memory, and DB connection pool utilisation.
fn collect_runtime_gauges(
    metrics: &Metrics,
    event_bus: &crate::ws::EventBus,
    system: &mut sysinfo::System,
) {
    metrics
        .ws_connections_active
        .set(event_bus.active_connections() as f64);

    system.refresh_memory();
    metrics.memory_total_bytes.set(system.total_memory() as f64);
    if let Ok(pid) = sysinfo::get_current_pid() {
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if let Some(process) = system.process(pid) {
            metrics.memory_used_bytes.set(process.memory() as f64);
        }
    }
}

/// Periodically refresh gauges that cannot be maintained incrementally.
///
/// Pool counts and windowed prediction volume are derived from the database
/// (so they stay correct across restarts and replicas); WebSocket and memory
/// gauges are sampled from the process. Runs until the task is aborted.
pub async fn run_collector_loop(
    pool: sqlx::PgPool,
    metrics: SharedMetrics,
    event_bus: crate::ws::EventBus,
    interval: Duration,
) {
    let pool_gauges = crate::db::metrics::register_pool_gauges(&metrics.registry)
        .map_err(|error| tracing::warn!(%error, "failed to register db pool gauges"))
        .ok();
    let mut system = sysinfo::System::new();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        collect_runtime_gauges(&metrics, &event_bus, &mut system);
        if let Some(gauges) = &pool_gauges {
            gauges.record(&pool);
        }
        if let Err(error) = collect_db_gauges(&pool, &metrics).await {
            tracing::warn!(%error, "metrics collector failed to refresh database gauges");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Metrics::new()` must succeed without panicking.
    #[test]
    fn metrics_new_does_not_panic() {
        let result = Metrics::new();
        assert!(result.is_ok(), "Metrics::new() must not return an error");
    }

    /// All expected metric names are registered in the Prometheus registry.
    #[test]
    fn metrics_registers_all_expected_metrics() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");

        // Gauge / Histogram scalar metrics appear immediately after registration.
        let families = metrics.registry.gather();
        let names: Vec<&str> = families.iter().map(|f| f.get_name()).collect();

        // Original scalar metrics
        assert!(names.contains(&"app_up"), "app_up must be registered");
        assert!(
            names.contains(&"app_build_info"),
            "app_build_info must be registered"
        );
        assert!(
            names.contains(&"app_memory_used_bytes"),
            "app_memory_used_bytes must be registered"
        );
        assert!(
            names.contains(&"app_price_cache_assets"),
            "app_price_cache_assets must be registered"
        );
        assert!(
            names.contains(&"app_price_cache_fetch_duration_seconds"),
            "app_price_cache_fetch_duration_seconds must be registered"
        );
        // New scalar metrics (gauges/counters without labels appear immediately)
        assert!(
            names.contains(&"app_ws_connections_active"),
            "app_ws_connections_active must be registered"
        );
        assert!(
            names.contains(&"app_resolved_pools"),
            "app_resolved_pools must be registered"
        );
        assert!(
            names.contains(&"app_cancelled_pools"),
            "app_cancelled_pools must be registered"
        );
        assert!(
            names.contains(&"app_worker_queue_depth"),
            "app_worker_queue_depth must be registered"
        );
        assert!(
            names.contains(&"app_worker_dlq_depth"),
            "app_worker_dlq_depth must be registered"
        );

        // HistogramVec and CounterVec metrics only appear in gather() after
        // at least one label set has been used — seed them now.
        metrics.record_db_query("test_query", "ok", 0.001);
        metrics.record_redis_op("get", "ok", 0.0005);
        metrics
            .http_requests_total
            .with_label_values(&["GET", "/health", "200"])
            .inc();

        let families = metrics.registry.gather();
        let names: Vec<&str> = families.iter().map(|f| f.get_name()).collect();

        assert!(
            names.contains(&"app_db_query_duration_seconds"),
            "app_db_query_duration_seconds must be registered after first use"
        );
        assert!(
            names.contains(&"app_redis_operation_duration_seconds"),
            "app_redis_operation_duration_seconds must be registered after first use"
        );
        assert!(
            names.contains(&"app_http_requests_total"),
            "app_http_requests_total must be registered after first use"
        );
    }

    /// `app_up` gauge is set to `1.0` immediately after initialization.
    #[test]
    fn metrics_app_up_is_one_after_init() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        assert_eq!(
            metrics.app_up.get(),
            1.0,
            "app_up must be 1.0 after initialization"
        );
    }

    /// `app_build_info` gauge is set to `1.0` immediately after initialization.
    #[test]
    fn metrics_app_info_is_one_after_init() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        assert_eq!(
            metrics.app_info.get(),
            1.0,
            "app_build_info must be 1.0 after initialization"
        );
    }

    /// `gather_text()` returns valid Prometheus text exposition format.
    #[test]
    fn metrics_gather_text_returns_valid_output() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        let text = metrics
            .gather_text()
            .expect("gather_text() must not return an error");

        assert!(
            text.contains("# HELP app_up"),
            "output must contain HELP comment for app_up"
        );
        assert!(
            text.contains("# TYPE app_up gauge"),
            "output must contain TYPE comment for app_up"
        );
        assert!(
            text.contains("app_up 1"),
            "output must contain app_up metric value"
        );
    }

    /// Calling `Metrics::new()` multiple times produces independent registries.
    #[test]
    fn metrics_new_can_be_called_multiple_times() {
        let m1 = Metrics::new().expect("first Metrics::new() must succeed");
        let m2 = Metrics::new().expect("second Metrics::new() must succeed");

        m1.http_requests_total
            .with_label_values(&["GET", "/health", "200"])
            .inc();

        let m2_text = m2.gather_text().expect("gather_text() must succeed");
        assert!(
            !m2_text.contains("GET"),
            "m2 registry must be independent of m1"
        );
    }

    /// `record_db_query` updates the histogram and counter.
    #[test]
    fn record_db_query_updates_metrics() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.record_db_query("get_pool", "ok", 0.005);

        let text = metrics.gather_text().expect("gather_text must succeed");
        assert!(
            text.contains("app_db_query_duration_seconds"),
            "db query histogram must appear in output"
        );
        assert!(
            text.contains("app_db_queries_total"),
            "db query counter must appear in output"
        );
    }

    /// `record_redis_op` updates the histogram and counter.
    #[test]
    fn record_redis_op_updates_metrics() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.record_redis_op("get", "ok", 0.001);

        let text = metrics.gather_text().expect("gather_text must succeed");
        assert!(
            text.contains("app_redis_operation_duration_seconds"),
            "redis op histogram must appear in output"
        );
    }

    /// WebSocket connect / disconnect helpers update the active gauge correctly.
    #[test]
    fn ws_connect_disconnect_updates_gauge() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        assert_eq!(metrics.ws_connections_active.get(), 0.0);

        metrics.ws_connect();
        metrics.ws_connect();
        assert_eq!(metrics.ws_connections_active.get(), 2.0);

        metrics.ws_disconnect();
        assert_eq!(metrics.ws_connections_active.get(), 1.0);
    }

    /// `set_pool_counts` updates all three pool state gauges.
    #[test]
    fn set_pool_counts_updates_all_gauges() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.set_pool_counts(10, 3, 1);

        assert_eq!(metrics.active_pools.get(), 10.0);
        assert_eq!(metrics.resolved_pools.get(), 3.0);
        assert_eq!(metrics.cancelled_pools.get(), 1.0);
    }

    /// `record_prediction` increments the monotonic count and volume.
    #[test]
    fn record_prediction_increments_counters() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.record_prediction(5_000_000);
        metrics.record_prediction(1_000_000);

        assert_eq!(metrics.predictions_total.get(), 2.0);
        assert_eq!(metrics.prediction_volume_stroops_total.get(), 6_000_000.0);
    }

    /// `set_prediction_window` publishes a gauge per window label.
    #[test]
    fn set_prediction_window_sets_gauges_for_every_window() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        for (i, (window, _)) in PREDICTION_WINDOWS.iter().enumerate() {
            metrics.set_prediction_window(window, i as u64 + 1, (i as u64 + 1) * 100);
        }

        let text = metrics.gather_text().expect("gather_text must succeed");
        for (window, _) in PREDICTION_WINDOWS {
            assert!(
                text.contains(&format!("app_predictions_window{{window=\"{window}\"}}")),
                "window '{window}' must appear in output"
            );
        }
        assert_eq!(
            metrics
                .prediction_volume_window_stroops
                .with_label_values(&["24h"])
                .get(),
            400.0
        );

        // Windows are gauges: a later refresh replaces rather than adds.
        metrics.set_prediction_window("1m", 0, 0);
        assert_eq!(
            metrics.predictions_window.with_label_values(&["1m"]).get(),
            0.0
        );
    }

    #[test]
    fn pool_state_bucket_maps_db_states() {
        assert_eq!(pool_state_bucket("active"), Some("active"));
        assert_eq!(pool_state_bucket("settled"), Some("resolved"));
        assert_eq!(pool_state_bucket("closed"), Some("cancelled"));
        assert_eq!(pool_state_bucket("unknown"), None);
    }

    #[test]
    fn ws_frame_and_drop_helpers_update_counters() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.record_ws_frame("batch", "deflate", 512);
        metrics.record_ws_dropped("lagged", 7);

        assert_eq!(
            metrics
                .ws_messages_sent_total
                .with_label_values(&["batch", "deflate"])
                .get(),
            1.0
        );
        assert_eq!(
            metrics
                .ws_bytes_sent_total
                .with_label_values(&["deflate"])
                .get(),
            512.0
        );
        assert_eq!(
            metrics
                .ws_events_dropped_total
                .with_label_values(&["lagged"])
                .get(),
            7.0
        );
    }

    #[tokio::test]
    async fn observe_db_passes_through_result() {
        let ok: Result<i32, &str> = observe_db("test_ok", async { Ok(7) }).await;
        assert_eq!(ok, Ok(7));
        let err: Result<i32, &str> = observe_db("test_err", async { Err("boom") }).await;
        assert_eq!(err, Err("boom"));
    }

    /// HTTP 500 responses are tracked via the general request counter.
    #[test]
    fn http_500_responses_are_counted_by_status_label() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics
            .http_requests_total
            .with_label_values(&["GET", "/health", "500"])
            .inc();

        let text = metrics.gather_text().expect("gather_text() must succeed");
        assert!(
            text.contains("app_http_requests_total"),
            "output must contain app_http_requests_total"
        );
    }

    /// `SharedMetrics` (Arc<Metrics>) can be cloned and used from multiple owners.
    #[test]
    fn shared_metrics_can_be_cloned() {
        let metrics: SharedMetrics = Arc::new(Metrics::new().expect("Metrics::new() must succeed"));
        let cloned = metrics.clone();

        cloned
            .http_requests_total
            .with_label_values(&["POST", "/api/v1/pools", "201"])
            .inc();

        let text = metrics
            .gather_text()
            .expect("gather_text() must succeed on original");
        assert!(
            text.contains("app_http_requests_total"),
            "original must see counter incremented via clone"
        );
    }

    /// `record_worker_queue_depth` updates queue depth and DLQ depth gauges.
    #[test]
    fn record_worker_queue_depth_updates_gauges() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        assert_eq!(metrics.worker_queue_depth.get(), 0.0);
        assert_eq!(metrics.worker_dlq_depth.get(), 0.0);

        metrics.record_worker_queue_depth(12, 3);
        assert_eq!(metrics.worker_queue_depth.get(), 12.0);
        assert_eq!(metrics.worker_dlq_depth.get(), 3.0);
    }
}
