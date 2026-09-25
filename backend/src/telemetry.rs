//! OpenTelemetry tracer provider setup for distributed tracing.
//!
//! This module initialises an OpenTelemetry tracer provider and wires it into
//! the `tracing` subscriber stack used by the Axum server.  Spans emitted via
//! `tracing::info_span!` and `#[tracing::instrument]` are forwarded to the
//! configured OTLP collector (Jaeger, Grafana Tempo, OpenTelemetry Collector…).
//!
//! # Environment variables
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `TELEMETRY_ENABLED` | `"false"` | Set to `"true"` to activate OTel export |
//! | `SERVICE_NAME` | `"predifi-backend"` | Reported service name |
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | `"http://localhost:4317"` | OTLP gRPC collector |
//! | `APP_ENV` | `"development"` | Deployment environment tag |
//!
//! Telemetry is **opt-in** (`TELEMETRY_ENABLED` defaults to `"false"`) so the
//! server starts cleanly in environments without a collector configured.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    resource::Resource,
    trace::{self, Tracer},
};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::Level;

// ── Global dynamic log-level state ─────────────────────────────────────────────

/// Numeric encoding of [`Level`] used by the atomic filter.
/// Lower numeric value = stricter (fewer events pass).
const LEVEL_ERROR: u8 = 1;
const LEVEL_WARN: u8 = 2;
const LEVEL_INFO: u8 = 3;
const LEVEL_DEBUG: u8 = 4;
const LEVEL_TRACE: u8 = 5;

/// Convert a [`Level`] to its numeric encoding.
fn level_to_u8(level: &Level) -> u8 {
    match *level {
        Level::ERROR => LEVEL_ERROR,
        Level::WARN => LEVEL_WARN,
        Level::INFO => LEVEL_INFO,
        Level::DEBUG => LEVEL_DEBUG,
        Level::TRACE => LEVEL_TRACE,
    }
}

/// Convert a numeric level back to [`Level`].
fn u8_to_level(value: u8) -> Level {
    match value {
        _ if value <= LEVEL_ERROR => Level::ERROR,
        _ if value <= LEVEL_WARN => Level::WARN,
        _ if value <= LEVEL_INFO => Level::INFO,
        _ if value <= LEVEL_DEBUG => Level::DEBUG,
        _ => Level::TRACE,
    }
}

/// Parse a log-level string into its numeric encoding.
pub fn parse_log_level(level: &str) -> Result<u8, String> {
    let parsed = match level.to_lowercase().as_str() {
        "error" => LEVEL_ERROR,
        "warn" => LEVEL_WARN,
        "info" => LEVEL_INFO,
        "debug" => LEVEL_DEBUG,
        "trace" => LEVEL_TRACE,
        _ => return Err(format!("invalid log level: {level}")),
    };
    Ok(parsed)
}

/// Process-wide atomic holding the current minimum log level.
///
/// Initialised by [`init_tracing_subscriber`] or [`init_plain_subscriber`].
/// Updated by [`try_reload_log_level`] when the admin endpoint changes the level.
static CURRENT_LOG_LEVEL: OnceLock<Arc<AtomicU8>> = OnceLock::new();

/// A [`tracing_subscriber::Layer`] that filters events by a shared atomic log level.
///
/// This is the outermost layer in the subscriber stack.  When `event_enabled`
/// returns `false`, the event is dropped and never reaches inner layers
/// (fmt, OTel, etc.), which is the standard mechanism for log-level filtering.
///
/// The level can be changed at runtime by updating [`CURRENT_LOG_LEVEL`].
#[derive(Clone)]
struct DynamicFilterLayer {
    _private: (),
}

impl DynamicFilterLayer {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for DynamicFilterLayer {
    fn event_enabled(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) -> bool {
        let Some(current) = CURRENT_LOG_LEVEL.get() else {
            return true;
        };
        let event_level = level_to_u8(event.metadata().level());
        let threshold = current.load(Ordering::Relaxed);
        event_level <= threshold
    }
}

// ── Resource builder ───────────────────────────────────────────────────────────

/// Build a [`Resource`] carrying standard service-identification attributes.
///
/// Every exported span carries `service.name`, `service.version`, and
/// `deployment.environment` so collectors can filter by service.
fn build_resource(service_name: &'static str) -> Resource {
    Resource::new(vec![
        KeyValue::new("service.name", service_name),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new(
            "deployment.environment",
            std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
        ),
    ])
}

// ── Provider initialisation ────────────────────────────────────────────────────

/// Initialise an OpenTelemetry tracer provider with an **OTLP gRPC** exporter.
///
/// Spans are exported asynchronously via a batch processor running on the
/// Tokio runtime.  The provider is registered as the global provider so any
/// code that calls `opentelemetry::global::tracer(…)` gets a tracer from the
/// same pipeline.
///
/// # Arguments
/// * `service_name`  – Reported service name (e.g. `"predifi-backend"`).
/// * `otlp_endpoint` – Collector gRPC endpoint (e.g. `"http://localhost:4317"`).
///
/// # Panics
/// Panics if the OTLP exporter cannot be constructed (bad endpoint string —
/// network failures are handled lazily by tonic).
///
/// # Example
/// ```no_run
/// use predifi_backend::telemetry::init_telemetry;
/// let tracer = init_telemetry("predifi-backend", "http://localhost:4317");
/// ```
pub fn init_telemetry(service_name: &'static str, otlp_endpoint: &'static str) -> Tracer {
    let resource = build_resource(service_name);

    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint)
        .build_span_exporter()
        .expect("failed to create OTLP span exporter");

    let provider = trace::TracerProvider::builder()
        .with_config(trace::Config::default().with_resource(resource))
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    let tracer = provider.tracer(service_name);
    opentelemetry::global::set_tracer_provider(provider);
    tracer
}

// ── Subscriber wiring ──────────────────────────────────────────────────────────

/// Initialise the `tracing` subscriber stack with an OpenTelemetry layer and
/// a reloadable log-level filter.
///
/// Replaces any previously installed global subscriber.  Callers **must not**
/// have already called `…::init()` on a subscriber before invoking this.
///
/// The stack (outer → inner):
/// 1. `DynamicFilterLayer`     — filters events by the current runtime log level.
/// 2. `tracing_opentelemetry::layer` — forwards spans to the OTel provider.
/// 3. `fmt::layer`             — console output (JSON or compact).
///
/// The log level can be changed at runtime via [`try_reload_log_level`]
/// without restarting.  It reverts to the configured default on restart
/// because the subscriber is rebuilt from the config value at startup.
///
/// # Arguments
/// * `tracer`    – Tracer obtained from [`init_telemetry`].
/// * `log_level` – `EnvFilter`-compatible string (e.g. `"info"`, `"debug"`).
/// * `use_json`  – `true` → newline-delimited JSON; `false` → compact human output.
pub fn init_tracing_subscriber(
    tracer: Tracer,
    log_level: &str,
    use_json: bool,
) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let numeric_level = parse_log_level(log_level).unwrap_or(LEVEL_INFO);
    let atomic = Arc::new(AtomicU8::new(numeric_level));
    let _ = CURRENT_LOG_LEVEL.set(atomic.clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let registry = tracing_subscriber::registry()
        .with(DynamicFilterLayer::new())
        .with(otel_layer);

    if use_json {
        registry.with(fmt_layer.json()).init();
    } else {
        registry.with(fmt_layer.compact()).init();
    }
}

/// Initialise a plain `tracing` subscriber without OpenTelemetry, using a
/// dynamic log-level filter.
///
/// Call this when `TELEMETRY_ENABLED` is not `"true"` so the admin endpoint
/// can still adjust verbosity without a restart.
pub fn init_plain_subscriber(log_level: &str, use_json: bool) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let numeric_level = parse_log_level(log_level).unwrap_or(LEVEL_INFO);
    let atomic = Arc::new(AtomicU8::new(numeric_level));
    let _ = CURRENT_LOG_LEVEL.set(atomic.clone());

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let registry = tracing_subscriber::registry()
        .with(DynamicFilterLayer::new());

    if use_json {
        registry.with(fmt_layer.json()).init();
    } else {
        registry.with(fmt_layer.compact()).init();
    }
}

// ── Graceful shutdown ──────────────────────────────────────────────────────────

/// Flush buffered spans and shut down the global OTel tracer provider.
///
/// Call this **after** all background workers are aborted and the HTTP server
/// has stopped, so no new spans are generated during the flush.
pub fn shutdown_tracer_provider() {
    opentelemetry::global::shutdown_tracer_provider();
}

// ── Runtime log-level control ──────────────────────────────────────────────────

/// Attempt to change the active log level at runtime.
///
/// The new level is applied immediately to all subsequent log events.  It
/// automatically reverts to the configured default on restart because the
/// subscriber is rebuilt from the config value at startup.
///
/// Returns `Err` if [`init_tracing_subscriber`] or [`init_plain_subscriber`]
/// has not yet been called, or if `new_level` is not a recognised level name.
pub fn try_reload_log_level(new_level: &str) -> Result<(), String> {
    let numeric = parse_log_level(new_level)?;
    let current = CURRENT_LOG_LEVEL
        .get()
        .ok_or_else(|| "tracing subscriber has not been initialised".to_string())?;
    current.store(numeric, Ordering::Relaxed);
    Ok(())
}

/// Return the currently active log level as a [`Level`].
pub fn current_log_level() -> Level {
    match CURRENT_LOG_LEVEL.get() {
        Some(atomic) => u8_to_level(atomic.load(Ordering::Relaxed)),
        None => Level::INFO,
    }
}

// ── Environment-driven bootstrap ───────────────────────────────────────────────

/// Initialise the OTel tracer provider from environment variables.
///
/// Returns `Some(Tracer)` when telemetry is active, `None` when disabled.
/// Callers should pass the tracer to [`init_tracing_subscriber`].
///
/// See the [module-level docs](self) for the full env var list.
pub fn init_telemetry_from_env() -> Option<Tracer> {
    let enabled = std::env::var("TELEMETRY_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    if !enabled {
        return None;
    }

    let service_name =
        std::env::var("SERVICE_NAME").unwrap_or_else(|_| "predifi-backend".to_string());

    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let name: &'static str = Box::leak(service_name.into_boxed_str());
    let endpoint: &'static str = Box::leak(otlp_endpoint.into_boxed_str());
    Some(init_telemetry(name, endpoint))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `parse_log_level` recognises all standard level names.
    #[test]
    fn parse_log_level_recognises_standard_names() {
        assert_eq!(parse_log_level("error").unwrap(), LEVEL_ERROR);
        assert_eq!(parse_log_level("warn").unwrap(), LEVEL_WARN);
        assert_eq!(parse_log_level("info").unwrap(), LEVEL_INFO);
        assert_eq!(parse_log_level("debug").unwrap(), LEVEL_DEBUG);
        assert_eq!(parse_log_level("trace").unwrap(), LEVEL_TRACE);
    }

    /// `parse_log_level` is case-insensitive.
    #[test]
    fn parse_log_level_is_case_insensitive() {
        assert_eq!(parse_log_level("INFO").unwrap(), LEVEL_INFO);
        assert_eq!(parse_log_level("Debug").unwrap(), LEVEL_DEBUG);
    }

    /// `parse_log_level` rejects unknown strings.
    #[test]
    fn parse_log_level_rejects_unknown() {
        assert!(parse_log_level("verbose").is_err());
        assert!(parse_log_level("").is_err());
    }

    /// `u8_to_level` maps numeric values back to `Level`.
    #[test]
    fn u8_to_level_maps_numeric_values() {
        assert_eq!(u8_to_level(LEVEL_ERROR), Level::ERROR);
        assert_eq!(u8_to_level(LEVEL_WARN), Level::WARN);
        assert_eq!(u8_to_level(LEVEL_INFO), Level::INFO);
        assert_eq!(u8_to_level(LEVEL_DEBUG), Level::DEBUG);
        assert_eq!(u8_to_level(LEVEL_TRACE), Level::TRACE);
        assert_eq!(u8_to_level(255), Level::TRACE);
        assert_eq!(u8_to_level(0), Level::ERROR);
    }

    /// `level_to_u8` maps `Level` to numeric values.
    #[test]
    fn level_to_u8_maps_levels() {
        assert_eq!(level_to_u8(&Level::ERROR), LEVEL_ERROR);
        assert_eq!(level_to_u8(&Level::WARN), LEVEL_WARN);
        assert_eq!(level_to_u8(&Level::INFO), LEVEL_INFO);
        assert_eq!(level_to_u8(&Level::DEBUG), LEVEL_DEBUG);
        assert_eq!(level_to_u8(&Level::TRACE), LEVEL_TRACE);
    }

    /// `TELEMETRY_ENABLED=false` must return `None` without any network call.
    #[test]
    fn telemetry_disabled_when_env_false() {
        std::env::set_var("TELEMETRY_ENABLED", "false");
        let result = init_telemetry_from_env();
        assert!(
            result.is_none(),
            "init_telemetry_from_env must return None when TELEMETRY_ENABLED=false"
        );
        std::env::remove_var("TELEMETRY_ENABLED");
    }

    /// Absent `TELEMETRY_ENABLED` must default to disabled.
    #[test]
    fn telemetry_disabled_by_default() {
        std::env::remove_var("TELEMETRY_ENABLED");
        let result = init_telemetry_from_env();
        assert!(
            result.is_none(),
            "init_telemetry_from_env must return None when TELEMETRY_ENABLED is unset"
        );
    }

    /// `build_resource` must embed the service name.
    #[test]
    fn build_resource_includes_service_name() {
        let resource = build_resource("test-service");
        let resource_str = format!("{resource:?}");
        assert!(
            resource_str.contains("test-service"),
            "resource must contain the service name, got: {resource_str}"
        );
    }

    /// `shutdown_tracer_provider` must not panic when no custom provider is registered.
    #[test]
    fn shutdown_tracer_provider_is_safe_with_no_provider() {
        shutdown_tracer_provider();
    }
}
