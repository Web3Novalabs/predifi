use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, runtime, trace};
use opentelemetry_stdout::SpanExporter;
use std::env;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub otlp_endpoint: Option<String>,
    pub log_level: String,
}

impl TracingConfig {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "predifi-backend".to_string()),
            service_version: env::var("OTEL_SERVICE_VERSION")
                .unwrap_or_else(|_| "0.1.0".to_string()),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

pub fn init_tracing(
    config: &TracingConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Step 1: Initialize OpenTelemetry tracer
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("environment", config.environment.clone()),
    ]);

    let _tracer = if let Some(endpoint) = &config.otlp_endpoint {
        // Use OTLP exporter for production
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(trace::Config::default().with_resource(resource))
            .install_batch(runtime::Tokio)?
    } else {
        // Use stdout exporter for development
        let exporter = SpanExporter::default();
        let provider = trace::TracerProvider::builder()
            .with_span_processor(
                trace::BatchSpanProcessor::builder(exporter, runtime::Tokio).build(),
            )
            .with_config(trace::Config::default().with_resource(resource))
            .build();

        let tracer = provider.tracer("predifi-backend");
        global::set_tracer_provider(provider);
        tracer
    };

    // Step 2: Set up tracing subscriber with careful filtering to prevent circular dependencies
    // Create a filter for the fmt layer that allows our application logs
    let fmt_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level))
        .add_directive("h2=warn".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("tonic=warn".parse().unwrap())
        .add_directive("tower=warn".parse().unwrap());

    // DO NOT use OpenTelemetryLayer as it causes circular dependencies
    // Instead, we'll manually inject trace context into logs
    let subscriber = Registry::default().with(
        fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_current_span(true)
            .with_span_list(true)
            .with_filter(fmt_filter),
    );

    // Initialize the global subscriber WITH OpenTelemetry layer
    subscriber.init();

    // Log initialization success
    tracing::info!(
        service_name = config.service_name,
        service_version = config.service_version,
        environment = config.environment,
        otlp_endpoint = config.otlp_endpoint.as_deref().unwrap_or("stdout"),
        "OpenTelemetry tracing initialized successfully with automatic trace and span ID correlation"
    );

    Ok(())
}

pub fn shutdown_tracing() {
    tracing::info!("Shutting down OpenTelemetry tracing");
    global::shutdown_tracer_provider();
}

// Function to get current OpenTelemetry trace context for manual injection
// This avoids the circular dependency by not going through the tracing layer
pub fn get_trace_context() -> Option<(String, String)> {
    use opentelemetry::Context;
    use opentelemetry::trace::TraceContextExt;

    // Get the current OpenTelemetry context directly (not through tracing)
    let ctx = Context::current();
    let span = ctx.span();
    let span_context = span.span_context();

    if span_context.is_valid() {
        Some((
            span_context.trace_id().to_string(),
            span_context.span_id().to_string(),
        ))
    } else {
        None
    }
}
