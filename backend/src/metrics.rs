use prometheus::{
    CounterVec, Encoder, Gauge, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
    TextEncoder,
};
use std::sync::Arc;

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

    // ── WebSocket ─────────────────────────────────────────────────────────────
    /// Current number of open WebSocket connections.
    pub ws_connections_active: Gauge,
    /// Total WebSocket connections accepted since startup.
    pub ws_connections_total: prometheus::Counter,
    /// Total WebSocket connections closed since startup.
    pub ws_disconnections_total: prometheus::Counter,

    // ── Pool state gauges ────────────────────────────────────────────────────
    /// Currently active (open) prediction pools.
    pub active_pools: Gauge,
    /// Currently resolved prediction pools.
    pub resolved_pools: Gauge,
    /// Currently cancelled prediction pools.
    pub cancelled_pools: Gauge,

    // ── Prediction volume per time window ────────────────────────────────────
    /// Counter of predictions created, labelled by `window`
    /// (e.g. "1m", "5m", "1h", "24h") — updated by the metrics scrape path
    /// or a background roller.  See `record_prediction`.
    pub predictions_total: CounterVec,
    /// Counter of prediction amounts in stroops by `window`.
    pub prediction_volume_stroops_total: CounterVec,
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
        let app_up =
            Gauge::with_opts(Opts::new("app_up", "Application availability status."))?;
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

        // ── Prediction volume per time window ─────────────────────────────────
        let predictions_total = CounterVec::new(
            Opts::new(
                "app_predictions_total",
                "Total number of predictions created, labelled by time window.",
            ),
            &["window"],
        )?;

        let prediction_volume_stroops_total = CounterVec::new(
            Opts::new(
                "app_prediction_volume_stroops_total",
                "Total prediction stake volume in stroops, labelled by time window.",
            ),
            &["window"],
        )?;

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
        registry.register(Box::new(ws_connections_active.clone()))?;
        registry.register(Box::new(ws_connections_total.clone()))?;
        registry.register(Box::new(ws_disconnections_total.clone()))?;
        registry.register(Box::new(active_pools.clone()))?;
        registry.register(Box::new(resolved_pools.clone()))?;
        registry.register(Box::new(cancelled_pools.clone()))?;
        registry.register(Box::new(predictions_total.clone()))?;
        registry.register(Box::new(prediction_volume_stroops_total.clone()))?;

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
            ws_connections_active,
            ws_connections_total,
            ws_disconnections_total,
            active_pools,
            resolved_pools,
            cancelled_pools,
            predictions_total,
            prediction_volume_stroops_total,
        })
    }

    // ── Domain helpers ────────────────────────────────────────────────────────

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
    /// This method increments *all* time-window buckets simultaneously; a
    /// separate background roller can reset individual windows on a schedule.
    /// `amount_stroops` is the stake amount in Stellar stroops.
    pub fn record_prediction(&self, amount_stroops: u64) {
        for window in &["1m", "5m", "1h", "24h"] {
            self.predictions_total
                .with_label_values(&[window])
                .inc();
            self.prediction_volume_stroops_total
                .with_label_values(&[window])
                .inc_by(amount_stroops as f64);
        }
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

    /// `record_prediction` increments all four time-window counters.
    #[test]
    fn record_prediction_increments_all_windows() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        metrics.record_prediction(5_000_000);

        let text = metrics.gather_text().expect("gather_text must succeed");
        assert!(
            text.contains("app_predictions_total"),
            "predictions_total must appear after first use"
        );
        // All four windows should appear
        for window in &["1m", "5m", "1h", "24h"] {
            assert!(
                text.contains(&format!("window=\"{window}\"")),
                "window label '{window}' must appear in output"
            );
        }
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
}
