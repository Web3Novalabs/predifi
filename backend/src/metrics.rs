use prometheus::{CounterVec, Encoder, Gauge, Opts, Registry, TextEncoder};
use std::sync::Arc;

/// Shared application metrics exposed to Prometheus.
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub http_requests_total: CounterVec,
    pub app_up: Gauge,
    pub app_info: Gauge,
    pub memory_used_bytes: Gauge,
    pub memory_total_bytes: Gauge,
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

        let http_requests_total = CounterVec::new(
            Opts::new(
                "app_http_requests_total",
                "Total number of HTTP requests served by the backend.",
            ),
            &["method", "path", "status"],
        )?;

        let app_up = Gauge::with_opts(Opts::new("app_up", "Application availability status."))?;
        app_up.set(1.0);

        let app_info = Gauge::with_opts(
            Opts::new("app_build_info", "Static metadata about the backend build.")
                .const_label("service", "predifi-backend")
                .const_label("version", env!("CARGO_PKG_VERSION")),
        )?;
        app_info.set(1.0);

        let memory_used_bytes = Gauge::with_opts(Opts::new("app_memory_used_bytes", "Memory used by the backend in bytes."))?;
        let memory_total_bytes = Gauge::with_opts(Opts::new("app_memory_total_bytes", "Total system memory in bytes."))?;

        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(app_up.clone()))?;
        registry.register(Box::new(app_info.clone()))?;
        registry.register(Box::new(memory_used_bytes.clone()))?;
        registry.register(Box::new(memory_total_bytes.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            app_up,
            app_info,
            memory_used_bytes,
            memory_total_bytes,
        })
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
    ///
    /// This is the primary acceptance criterion for issue #966: verify that
    /// metrics are initialized without panicking.
    #[test]
    fn metrics_new_does_not_panic() {
        let result = Metrics::new();
        assert!(result.is_ok(), "Metrics::new() must not return an error");
    }

    /// All expected metric names are registered in the Prometheus registry.
    #[test]
    fn metrics_registers_all_expected_metrics() {
        let metrics = Metrics::new().expect("Metrics::new() must succeed");
        let families = metrics.registry.gather();
        let names: Vec<&str> = families.iter().map(|f| f.get_name()).collect();

        assert!(
            names.contains(&"app_http_requests_total"),
            "app_http_requests_total must be registered"
        );
        assert!(
            names.contains(&"app_up"),
            "app_up must be registered"
        );
        assert!(
            names.contains(&"app_build_info"),
            "app_build_info must be registered"
        );
        assert!(
            names.contains(&"app_memory_used_bytes"),
            "app_memory_used_bytes must be registered"
        );
        assert!(
            names.contains(&"app_memory_total_bytes"),
            "app_memory_total_bytes must be registered"
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

    /// Calling `Metrics::new()` multiple times produces independent registries
    /// (no duplicate-registration panic).
    #[test]
    fn metrics_new_can_be_called_multiple_times() {
        let m1 = Metrics::new().expect("first Metrics::new() must succeed");
        let m2 = Metrics::new().expect("second Metrics::new() must succeed");

        // Increment a counter on m1 and verify m2 is unaffected.
        m1.http_requests_total
            .with_label_values(&["GET", "/health", "200"])
            .inc();

        let m2_text = m2.gather_text().expect("gather_text() must succeed");
        // m2's counter should not appear in the output (zero counters are omitted).
        assert!(
            !m2_text.contains("GET"),
            "m2 registry must be independent of m1"
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
