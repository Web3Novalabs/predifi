use prometheus::{CounterVec, Encoder, Gauge, Opts, Registry, TextEncoder};
use std::sync::Arc;
use sysinfo::{System, SystemExt};

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

    /// Refresh the memory gauge metrics from the current system state.
    ///
    /// Reads live memory figures via [`sysinfo`] and updates
    /// `app_memory_used_bytes` and `app_memory_total_bytes`. Call this
    /// periodically (e.g. from a background task) to keep the gauges current.
    pub fn update_memory_metrics(&self) {
        let mut sys = System::new_all();
        sys.refresh_memory();
        self.memory_used_bytes.set(sys.used_memory() as f64);
        self.memory_total_bytes.set(sys.total_memory() as f64);
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
