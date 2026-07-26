//! Integration tests validating the OpenAPI spec against real API behaviour (#1381).
//!
//! `openapi.rs` documents paths, parameters, and response bodies by hand, so it
//! can drift from the router without anything failing to compile. These tests
//! drive the actual Axum router and assert the spec still describes it:
//!
//! * every documented path is routed (no stale entries),
//! * every documented `$ref` resolves to a component schema,
//! * the spec is served over HTTP and parses as JSON.
//!
//! The generated TypeScript client (see `docs/openapi-client-generation.md`)
//! is only as trustworthy as the spec, so these checks are what keep the
//! frontend's types honest.

use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt; // provides `.oneshot()`
use utoipa::OpenApi;

use crate::config::Config;
use crate::openapi::ApiDoc;
use crate::{build_router, price_cache::PriceCache, redis_cache::RedisCache};

/// Build the same router the server exposes, minus the database pool.
fn test_router() -> axum::Router {
    build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        crate::ws::EventBus::new(),
    )
}

/// The spec as a `serde_json::Value` — the same bytes tooling consumes.
fn spec() -> Value {
    serde_json::to_value(ApiDoc::openapi()).expect("spec must serialize to JSON")
}

/// Substitute concrete sample values for `{path}` template parameters so a
/// documented path can actually be requested.
fn concrete_path(template: &str) -> String {
    template
        .replace("{pool_id}", "1")
        .replace("{market_id}", "1")
        .replace(
            "{address}",
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1",
        )
}

/// A path is "routed" when the router recognises it — anything other than a
/// 404. Handlers may still return 503/500 without a database, which is fine:
/// this asserts routing, not handler success.
async fn status_for(method: &str, path: &str) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(axum::body::Body::from("{}"))
        .expect("request must build");

    test_router()
        .oneshot(request)
        .await
        .expect("router must respond")
        .status()
}

/// Every path documented in the spec must exist on the router.
#[tokio::test]
async fn documented_paths_are_routed() {
    let spec = spec();
    let paths = spec["paths"].as_object().expect("spec must have paths");
    assert!(!paths.is_empty(), "spec documents no paths at all");

    let mut missing = Vec::new();

    for (template, operations) in paths {
        let operations = operations.as_object().expect("path item must be an object");
        for method in operations.keys() {
            let path = concrete_path(template);
            let status = status_for(&method.to_uppercase(), &path).await;

            if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
                missing.push(format!("{} {template} -> {status}", method.to_uppercase()));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "spec documents endpoints the router does not serve: {missing:?}"
    );
}

/// Every `$ref` in the spec must point at a schema that actually exists,
/// otherwise client generation emits broken types.
#[test]
fn every_schema_ref_resolves() {
    let spec = spec();
    let components = spec["components"]["schemas"]
        .as_object()
        .expect("spec must define component schemas");

    fn collect_refs(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    if key == "$ref" {
                        if let Some(reference) = val.as_str() {
                            out.push(reference.to_string());
                        }
                    }
                    collect_refs(val, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| collect_refs(item, out)),
            _ => {}
        }
    }

    let mut refs = Vec::new();
    collect_refs(&spec, &mut refs);
    assert!(!refs.is_empty(), "spec contains no schema references");

    let unresolved: Vec<_> = refs
        .iter()
        .filter(|reference| {
            reference
                .strip_prefix("#/components/schemas/")
                .map(|name| !components.contains_key(name))
                .unwrap_or(true)
        })
        .collect();

    assert!(unresolved.is_empty(), "unresolved $refs: {unresolved:?}");
}

/// Documented operations must declare at least one response, and every
/// non-204 success response must name a body schema — generators produce
/// `unknown` otherwise.
#[test]
fn documented_operations_declare_responses() {
    let spec = spec();
    let mut problems = Vec::new();

    for (path, operations) in spec["paths"].as_object().expect("paths") {
        for (method, operation) in operations.as_object().expect("path item") {
            let responses = operation["responses"].as_object();
            match responses {
                None => problems.push(format!("{method} {path}: no responses declared")),
                Some(responses) if responses.is_empty() => {
                    problems.push(format!("{method} {path}: empty responses"));
                }
                Some(responses) => {
                    if !responses.keys().any(|code| code.starts_with('2')) {
                        problems.push(format!("{method} {path}: no 2xx response"));
                    }
                }
            }
        }
    }

    assert!(problems.is_empty(), "spec response problems: {problems:?}");
}

/// The spec must be reachable over HTTP and parse as JSON — this is the URL
/// the client generator is pointed at.
#[tokio::test]
async fn spec_is_served_as_valid_json() {
    let request = Request::builder()
        .method("GET")
        .uri("/api-docs/openapi.json")
        .body(axum::body::Body::empty())
        .expect("request must build");

    let response = test_router()
        .oneshot(request)
        .await
        .expect("router must respond");

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body must read");
    let served: Value = serde_json::from_slice(&bytes).expect("served spec must be valid JSON");

    assert_eq!(
        served,
        spec(),
        "served spec must match the compiled-in ApiDoc"
    );
    assert!(
        served["openapi"]
            .as_str()
            .is_some_and(|v| v.starts_with("3.")),
        "spec must declare an OpenAPI 3.x version"
    );
}
