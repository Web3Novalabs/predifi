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
use std::sync::{OnceLock, RwLock};
use tracing::Level;
use tracing_subscriber::EnvFilter;

// ── Per-module log filtering ───────────────────────────────────────────────────

/// Numeric encoding of the standard level names accepted in directives.
/// Lower numeric value = stricter (fewer events pass).
const LEVEL_ERROR: u8 = 1;
const LEVEL_WARN: u8 = 2;
const LEVEL_INFO: u8 = 3;
const LEVEL_DEBUG: u8 = 4;
const LEVEL_TRACE: u8 = 5;

/// Directive used when the configured filter is empty or invalid.
pub const DEFAULT_LOG_FILTER: &str = "info";

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

fn is_valid_level(level: &str) -> bool {
    level.eq_ignore_ascii_case("off") || parse_log_level(level).is_ok()
}

/// Validate a comma-separated list of filter directives.
///
/// Each directive is either a bare level (`info`) that sets the default, or
/// `module::path=level` that overrides it for one module, e.g.
/// `info,predifi_backend::ws=debug,sqlx=warn`. `EnvFilter` itself would
/// accept a bare word such as `verbose` as a *target* name, silently turning
/// a typo into "log nothing new"; this check rejects that instead.
pub fn validate_log_filter(directives: &str) -> Result<(), String> {
    let mut seen = 0;
    for directive in directives.split(',').map(str::trim) {
        if directive.is_empty() {
            continue;
        }
        seen += 1;
        match directive.split_once('=') {
            None if is_valid_level(directive) => {}
            None => return Err(format!("invalid log level: {directive}")),
            Some((target, level)) => {
                let target_ok = !target.is_empty()
                    && target
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '-'));
                if !target_ok {
                    return Err(format!("invalid log target in directive: {directive}"));
                }
                if !is_valid_level(level) {
                    return Err(format!("invalid log level in directive: {directive}"));
                }
            }
        }
    }
    if seen == 0 {
        return Err("log filter must not be empty".to_string());
    }
    Ok(())
}

/// Trim whitespace around each directive and drop empty ones
/// (`" info , ws=debug,"` → `"info,ws=debug"`); `EnvFilter` does not.
pub fn normalize_log_filter(directives: &str) -> String {
    directives
        .split(',')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

/// Build an [`EnvFilter`] from validated directives.
pub fn build_env_filter(directives: &str) -> Result<EnvFilter, String> {
    validate_log_filter(directives)?;
    EnvFilter::builder()
        .parse(normalize_log_filter(directives))
        .map_err(|error| format!("invalid log filter: {error}"))
}

/// Build the startup filter, falling back to [`DEFAULT_LOG_FILTER`] (with a
/// note on stderr — tracing is not up yet) when `directives` is invalid.
fn initial_filter(directives: &str) -> (EnvFilter, String) {
    match build_env_filter(directives) {
        Ok(filter) => (filter, normalize_log_filter(directives)),
        Err(error) => {
            eprintln!("{error}; falling back to RUST_LOG={DEFAULT_LOG_FILTER}");
            (
                EnvFilter::new(DEFAULT_LOG_FILTER),
                DEFAULT_LOG_FILTER.to_string(),
            )
        }
    }
}

type FilterReloader = Box<dyn Fn(EnvFilter) -> Result<(), String> + Send + Sync>;

/// Swaps the active filter; installed by the `init_*_subscriber` functions.
static FILTER_RELOADER: OnceLock<FilterReloader> = OnceLock::new();

/// Directives of the currently active filter.
static CURRENT_FILTER: RwLock<String> = RwLock::new(String::new());

fn set_current_filter(directives: &str) {
    let mut current = CURRENT_FILTER.write().unwrap_or_else(|e| e.into_inner());
    *current = directives.to_string();
}

/// Output format of the console log layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Newline-delimited JSON (production default).
    Json,
    /// Compact human-readable lines for local development.
    Pretty,
}

impl LogFormat {
    /// Read `PREDIFI_LOG_FORMAT` (`json` | `pretty`), defaulting to JSON.
    pub fn from_env() -> Self {
        match std::env::var("PREDIFI_LOG_FORMAT").as_deref() {
            Ok("pretty") | Ok("text") | Ok("compact") => Self::Pretty,
            _ => Self::Json,
        }
    }
}

/// Install the subscriber stack: reloadable per-module `EnvFilter`, optional
/// OTel layer, and a JSON or compact console layer.
///
/// The JSON layer flattens event fields to the top level and includes the
/// event's target and the full list of enclosing spans, so every line carries
/// the `correlation_id`, `user_address` and `pool_id` recorded on the
/// request span.
fn init_subscriber(tracer: Option<Tracer>, directives: &str, format: LogFormat) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let (filter, active) = initial_filter(directives);
    let (filter_layer, handle) = tracing_subscriber::reload::Layer::new(filter);
    let _ = FILTER_RELOADER.set(Box::new(move |filter| {
        handle
            .reload(filter)
            .map_err(|error| format!("failed to reload log filter: {error}"))
    }));
    set_current_filter(&active);

    let otel_layer = tracer.map(|tracer| tracing_opentelemetry::layer().with_tracer(tracer));
    let registry = tracing_subscriber::registry()
        .with(filter_layer)
        .with(otel_layer);

    let result = match format {
        LogFormat::Json => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_target(true),
            )
            .try_init(),
        LogFormat::Pretty => registry
            .with(tracing_subscriber::fmt::layer().compact().with_target(true))
            .try_init(),
    };
    if let Err(error) = result {
        eprintln!("tracing subscriber already initialised: {error}");
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
/// a reloadable per-module log filter.
///
/// The stack (outer → inner):
/// 1. reloadable `EnvFilter`   — per-module levels from `log_filter`.
/// 2. `tracing_opentelemetry::layer` — forwards spans to the OTel provider.
/// 3. `fmt::layer`             — console output (JSON or compact).
///
/// The filter can be changed at runtime via [`try_reload_log_level`]
/// without restarting. It reverts to the configured value on restart.
///
/// # Arguments
/// * `tracer`     – Tracer obtained from [`init_telemetry`].
/// * `log_filter` – `RUST_LOG`-style directives, e.g. `"info,predifi_backend::ws=debug"`.
/// * `use_json`   – `true` → newline-delimited JSON; `false` → compact human output.
pub fn init_tracing_subscriber(tracer: Tracer, log_filter: &str, use_json: bool) {
    let format = if use_json {
        LogFormat::Json
    } else {
        LogFormat::Pretty
    };
    init_subscriber(Some(tracer), log_filter, format);
}

/// Initialise a plain `tracing` subscriber without OpenTelemetry, using a
/// reloadable per-module log filter.
///
/// Call this when `TELEMETRY_ENABLED` is not `"true"` so the admin endpoint
/// can still adjust verbosity without a restart.
pub fn init_plain_subscriber(log_filter: &str, use_json: bool) {
    let format = if use_json {
        LogFormat::Json
    } else {
        LogFormat::Pretty
    };
    init_subscriber(None, log_filter, format);
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

/// Replace the active log filter at runtime.
///
/// `directives` takes the same syntax as `RUST_LOG`: a bare level
/// (`"debug"`) or per-module overrides
/// (`"info,predifi_backend::worker=debug,sqlx=warn"`). The new filter applies
/// immediately and reverts to the configured value on restart.
///
/// Returns `Err` if the subscriber has not been initialised or the directives
/// are invalid.
pub fn try_reload_log_level(directives: &str) -> Result<(), String> {
    let filter = build_env_filter(directives)?;
    let reload = FILTER_RELOADER
        .get()
        .ok_or_else(|| "tracing subscriber has not been initialised".to_string())?;
    reload(filter)?;
    set_current_filter(&normalize_log_filter(directives));
    Ok(())
}

/// Directives of the currently active log filter (empty before init).
pub fn current_log_filter() -> String {
    CURRENT_FILTER
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// The most verbose level enabled by any directive of the active filter.
pub fn current_log_level() -> Level {
    build_env_filter(&current_log_filter())
        .ok()
        .and_then(|filter| filter.max_level_hint())
        .and_then(|hint| hint.into_level())
        .unwrap_or(Level::INFO)
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

    /// Bare levels and per-module directives are accepted.
    #[test]
    fn validate_log_filter_accepts_per_module_directives() {
        for ok in [
            "info",
            "DEBUG",
            "off",
            "info,predifi_backend::ws=debug",
            "warn, predifi_backend::request_logger=error ,sqlx=off",
            "predifi_backend=trace",
        ] {
            assert!(validate_log_filter(ok).is_ok(), "{ok} should be valid");
            assert!(build_env_filter(ok).is_ok(), "{ok} should build");
        }
    }

    /// Typos that `EnvFilter` would treat as target names are rejected.
    #[test]
    fn validate_log_filter_rejects_bad_directives() {
        for bad in [
            "",
            " , ",
            "verbose",
            "info,ws=loud",
            "=debug",
            "foo bar=info",
            "x[span]=info",
        ] {
            assert!(
                validate_log_filter(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    /// An invalid startup filter falls back to the default.
    #[test]
    fn initial_filter_falls_back_on_invalid_input() {
        let (_, active) = initial_filter("nonsense");
        assert_eq!(active, DEFAULT_LOG_FILTER);
        let (_, active) = initial_filter(" info , predifi_backend::ws=debug,");
        assert_eq!(active, "info,predifi_backend::ws=debug");
    }

    /// The max level hint reflects the most verbose module override.
    #[test]
    fn env_filter_max_level_reflects_module_overrides() {
        let filter = build_env_filter("warn,predifi_backend::ws=debug").unwrap();
        assert_eq!(
            filter.max_level_hint().and_then(|h| h.into_level()),
            Some(Level::DEBUG)
        );
    }

    #[test]
    fn log_format_defaults_to_json() {
        std::env::remove_var("PREDIFI_LOG_FORMAT");
        assert_eq!(LogFormat::from_env(), LogFormat::Json);
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
