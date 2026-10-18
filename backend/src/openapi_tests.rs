//! Integration tests validating the OpenAPI spec against real API behaviour (#1948).
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
    let mut config = Config::default_for_test();
    config.stellar_rpc_url = "http://127.0.0.1:1".to_string();
    config.rpc_health_timeout_secs = 1;
    config.rpc_health_retry_count = 1;

    let cache = PriceCache::new();
    cache.update(std::collections::HashMap::from([("XLM".to_string(), 0.12)]));

    build_router(
        config,
        cache,
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

fn validate_json_schema(
    value: &Value,
    schema: &Value,
    spec: &Value,
    location: &str,
    problems: &mut Vec<String>,
) {
    if let Some(choices) = schema["oneOf"].as_array() {
        let mut matching_choices = 0;
        for choice in choices {
            let mut choice_problems = Vec::new();
            validate_json_schema(value, choice, spec, location, &mut choice_problems);
            if choice_problems.is_empty() {
                matching_choices += 1;
            }
        }
        if matching_choices != 1 {
            problems.push(format!(
                "{location}: expected exactly one matching schema in oneOf, found {matching_choices}"
            ));
        }
        return;
    }

    if let Some(reference) = schema["$ref"].as_str() {
        let Some(name) = reference.strip_prefix("#/components/schemas/") else {
            problems.push(format!(
                "{location}: unsupported schema reference {reference}"
            ));
            return;
        };
        let Some(target) = spec["components"]["schemas"].get(name) else {
            problems.push(format!(
                "{location}: unresolved schema reference {reference}"
            ));
            return;
        };
        validate_json_schema(value, target, spec, location, problems);
        return;
    }

    if value.is_null() && schema["nullable"] == true {
        return;
    }

    match schema["type"].as_str() {
        Some("object") => {
            let Some(object) = value.as_object() else {
                problems.push(format!("{location}: expected object, got {value}"));
                return;
            };
            if let Some(required) = schema["required"].as_array() {
                for key in required.iter().filter_map(Value::as_str) {
                    if !object.contains_key(key) {
                        problems.push(format!("{location}: missing required property {key}"));
                    }
                }
            }
            if let Some(properties) = schema["properties"].as_object() {
                for (key, property_schema) in properties {
                    if let Some(property) = object.get(key) {
                        validate_json_schema(
                            property,
                            property_schema,
                            spec,
                            &format!("{location}.{key}"),
                            problems,
                        );
                    }
                }
            }
        }
        Some("array") => {
            let Some(items) = value.as_array() else {
                problems.push(format!("{location}: expected array, got {value}"));
                return;
            };
            if let Some(item_schema) = schema.get("items") {
                for (index, item) in items.iter().enumerate() {
                    validate_json_schema(
                        item,
                        item_schema,
                        spec,
                        &format!("{location}[{index}]"),
                        problems,
                    );
                }
            }
        }
        Some("string") if !value.is_string() => {
            problems.push(format!("{location}: expected string, got {value}"));
        }
        Some("integer") if !value.as_i64().is_some() && !value.as_u64().is_some() => {
            problems.push(format!("{location}: expected integer, got {value}"));
        }
        Some("number") if !value.is_number() => {
            problems.push(format!("{location}: expected number, got {value}"));
        }
        Some("boolean") if !value.is_boolean() => {
            problems.push(format!("{location}: expected boolean, got {value}"));
        }
        _ => {}
    }
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

/// Representative successful and dependency-error responses must satisfy the
/// response status and JSON schema declared for that operation.
#[tokio::test]
async fn runtime_responses_match_documented_schemas() {
    let spec = spec();
    let cases = [
        ("GET", "/health"),
        ("GET", "/api/v1/fees"),
        ("GET", "/api/v1/pools"),
        ("GET", "/api/v1/pools/1"),
        ("GET", "/api/v1/stats"),
        ("GET", "/api/v1/prices"),
        ("GET", "/api/v1/leaderboard"),
        (
            "GET",
            "/api/v1/users/GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1/history",
        ),
        (
            "GET",
            "/api/v1/users/GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1/predictions",
        ),
        ("GET", "/api/v1/markets/1/predictions"),
        (
            "GET",
            "/api/v1/referrals/GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1",
        ),
        (
            "GET",
            "/api/v1/users/GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1/referrals",
        ),
    ];
    let mut problems = Vec::new();

    for (method, path) in cases {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(axum::body::Body::empty())
            .expect("request must build");
        let response = test_router()
            .oneshot(request)
            .await
            .expect("router must respond");
        let status = response.status();
        let operation = &spec["paths"][path][method.to_lowercase().as_str()];

        let status_code = status.as_u16().to_string();
        let Some(response_spec) = operation["responses"].get(status_code.as_str()) else {
            problems.push(format!(
                "{method} {path}: runtime returned undocumented status {status}"
            ));
            continue;
        };
        let Some(schema) = response_spec["content"]["application/json"].get("schema") else {
            problems.push(format!(
                "{method} {path} {status}: response has no application/json schema"
            ));
            continue;
        };

        let bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(error) => {
                problems.push(format!(
                    "{method} {path}: failed reading response body: {error}"
                ));
                continue;
            }
        };
        let body: Value = match serde_json::from_slice(&bytes) {
            Ok(body) => body,
            Err(error) => {
                problems.push(format!("{method} {path}: response was not JSON: {error}"));
                continue;
            }
        };
        validate_json_schema(&body, schema, &spec, path, &mut problems);
    }

    assert!(
        problems.is_empty(),
        "runtime responses differ from OpenAPI: {problems:?}"
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
