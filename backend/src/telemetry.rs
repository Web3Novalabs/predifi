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

// ── Resource builder ──────────────────────────────────────────────────────────

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

// ── Provider initialisation ───────────────────────────────────────────────────

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

    // Wire the resource so every span carries service.name / service.version /
    // deployment.environment attributes.
    let provider = trace::TracerProvider::builder()
        .with_config(trace::Config::default().with_resource(resource))
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    let tracer = provider.tracer(service_name);

    // Register as the process-wide global provider.
    opentelemetry::global::set_tracer_provider(provider);

    tracer
}

// ── Subscriber wiring ─────────────────────────────────────────────────────────

/// Initialise the `tracing` subscriber stack with an OpenTelemetry layer.
///
/// Replaces any previously installed global subscriber.  Callers **must not**
/// have already called `…::init()` on a subscriber before invoking this.
///
/// The stack (outer → inner):
/// 1. `EnvFilter`                    — honours `RUST_LOG` / the supplied `log_level`.
/// 2. `tracing_opentelemetry::layer` — forwards spans to the OTel provider.
/// 3. `fmt::layer`                   — console output (JSON or compact).
///
/// # Arguments
/// * `tracer`    – Tracer obtained from [`init_telemetry`].
/// * `log_level` – `EnvFilter`-compatible string (e.g. `"info"`, `"debug"`).
/// * `use_json`  – `true` → newline-delimited JSON; `false` → compact human output.
pub fn init_tracing_subscriber(tracer: Tracer, log_level: &str, use_json: bool) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::new(log_level);
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(otel_layer);

    if use_json {
        registry.with(fmt_layer.json()).init();
    } else {
        registry.with(fmt_layer.compact()).init();
    }
}

// ── Graceful shutdown ─────────────────────────────────────────────────────────

/// Flush buffered spans and shut down the global OTel tracer provider.
///
/// Call this **after** all background workers are aborted and the HTTP server
/// has stopped, so no new spans are generated during the flush.
pub fn shutdown_tracer_provider() {
    opentelemetry::global::shutdown_tracer_provider();
}

// ── Environment-driven bootstrap ─────────────────────────────────────────────

/// Initialise the OTel tracer provider from environment variables.
///
/// Returns `Some(Tracer)` when telemetry is active, `None` when disabled.
/// Callers should pass the tracer to [`init_tracing_subscriber`].
///
/// See the [module-level docs](self) for the full env var list.
pub fn init_telemetry_from_env() -> Option<Tracer> {
    // Telemetry is **opt-in**: absent or non-"true" values leave it disabled so
    // the server starts cleanly without a collector.
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

    // Box::leak gives `&'static str` from owned Strings coming from the env.
    let name: &'static str = Box::leak(service_name.into_boxed_str());
    let endpoint: &'static str = Box::leak(otlp_endpoint.into_boxed_str());
    Some(init_telemetry(name, endpoint))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Absent `TELEMETRY_ENABLED` must default to disabled (opt-in behaviour).
    #[test]
    fn telemetry_disabled_by_default() {
        std::env::remove_var("TELEMETRY_ENABLED");
        let result = init_telemetry_from_env();
        assert!(
            result.is_none(),
            "init_telemetry_from_env must return None when TELEMETRY_ENABLED is unset"
        );
    }

    /// `build_resource` must embed the service name in the resource attributes.
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
        // The global noop provider is always present; shutdown is a harmless no-op.
        shutdown_tracer_provider();
    }
}
